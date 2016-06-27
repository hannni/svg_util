//! Parsing and printing path data.
//!
//! Path data is used in the `d` attribute of `path` elements.
//! This module is about parsing and printing the segments, represented as
//! `PathSeg`.
//! `PathSeg`s map directly to all the different variants of segments from the
//! SVG specification.
//! If you want to do rendering or transformations working on `Primitive`s 
//! from the `primitive` module is probably a better idea.

use primitive::Primitive;
use util::{TokenWritten, DropLeadingZero, DropTrailingZeros, Count};

use std::fmt;
use std::fmt::{Write, Display};
use std::iter::Iterator;
use std::marker::PhantomData;
use std::str::from_utf8_unchecked;
use std::str::FromStr;
use std::ops::{Add, Sub};
use std::convert::From;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
/// The different types of `PathSeg`s.
///
/// The `u8` representation using the letters should reduce the amount of conversion that needs to
/// be done.
/// But this actually needs to get tested.
enum PathSegType {
    ClosepathAbs = b'Z',
    ClosepathRel = b'z',
    MovetoAbs = b'M',
    MovetoRel = b'm',
    LinetoAbs = b'L',
    LinetoRel = b'l',
    CurvetoCubicAbs = b'C',
    CurvetoCubicRel = b'c',
    CurvetoQuadraticAbs = b'Q',
    CurvetoQuadraticRel = b'q',
    ArcAbs = b'A',
    ArcRel = b'a',
    LinetoHorizontalAbs = b'H',
    LinetoHorizontalRel = b'h',
    LinetoVerticalAbs = b'V',
    LinetoVerticalRel = b'v',
    CurvetoCubicSmoothAbs = b'S',
    CurvetoCubicSmoothRel = b's',
    CurvetoQuadraticSmoothAbs = b'T',
    CurvetoQuadraticSmoothRel = b't',
}

impl PathSegType {
    fn from_u8(c: u8) -> Option<PathSegType> {
        match c {
            b'Z' => Some(PathSegType::ClosepathAbs),
            b'z' => Some(PathSegType::ClosepathRel),
            b'M' => Some(PathSegType::MovetoAbs),
            b'm' => Some(PathSegType::MovetoRel),
            b'L' => Some(PathSegType::LinetoAbs),
            b'l' => Some(PathSegType::LinetoRel),
            b'C' => Some(PathSegType::CurvetoCubicAbs),
            b'c' => Some(PathSegType::CurvetoCubicRel),
            b'Q' => Some(PathSegType::CurvetoQuadraticAbs),
            b'q' => Some(PathSegType::CurvetoQuadraticRel),
            b'A' => Some(PathSegType::ArcAbs),
            b'a' => Some(PathSegType::ArcRel),
            b'H' => Some(PathSegType::LinetoHorizontalAbs),
            b'h' => Some(PathSegType::LinetoHorizontalRel),
            b'V' => Some(PathSegType::LinetoVerticalAbs),
            b'v' => Some(PathSegType::LinetoVerticalRel),
            b'S' => Some(PathSegType::CurvetoCubicSmoothAbs),
            b's' => Some(PathSegType::CurvetoCubicSmoothRel),
            b'T' => Some(PathSegType::CurvetoQuadraticSmoothAbs),
            b't' => Some(PathSegType::CurvetoQuadraticSmoothRel),
            _ => None,
        }
    }
}

impl From<PathSegType> for char {
    fn from(pst: PathSegType) -> char {
        pst as u8 as char
    }
}

#[test]
// When converting a char to the `PathSegType` and converting it back it should still be the same
// char.
fn path_seg_type_test() {
    for i in 0..257 {
        let c0 = i as u8 as char;
        match i as u8 {
            b'A'...b'Z' | b'a'...b'z' => {
                if let Some(path_seg_type) = PathSegType::from_u8(i as u8) {
                    let c1 = path_seg_type.into();
                    assert_eq!(c0, c1);
                }
            }
            // All non letters shouldn't convert to a `PathSegType`
            non_letter => {
                assert_eq!(PathSegType::from_u8(non_letter), None);
            }
        }
    }
}

/// One segment of a path.
///
/// Note that repititions without a new character to define the type of a path segment get treated
/// as multiple `PathSeg`s.
/// E.g.: `l 1,1 1,1` and `l 1,1 l 1,1` are both parsed as two `PathSeg`s.
///
/// The different types of segments are explained in detail in the [SVG Specification](http://www.w3.org/TR/SVG11/paths.html#PathData)
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum PathSeg<T> {
    /// Closes the path and moves back to where the last moveto started the subpath.
    Closepath,
    /// Starts a new subpath at an absolute position.
    MovetoAbs((T, T)),
    /// Starts a new subpath at a position relative to the last current position.
    MovetoRel((T, T)), 
    /// Draws a line from the current position to an absolute position.
    LinetoAbs((T, T)),
    /// Draws a line from the current position to a position relative to the current position.
    LinetoRel((T, T)),
    /// Draws a cubic Bézier curve with absolute positions for first control point, second control point and end position.
    CurvetoCubicAbs((T, T), (T, T), (T, T)),
    /// Draws a cubic Bézier curve with relative positions for first control point, second control point and end position.
    CurvetoCubicRel((T, T), (T, T), (T, T)),
    /// Draws a quadratic Bézier curve with absolute positions for the control point and the end position.
    CurvetoQuadraticAbs((T, T), (T, T)),
    /// Draws a quadratic Bézier curve with positions relative to the start position.
    CurvetoQuadraticRel((T, T), (T, T)),
    /// Draws an arc with radius in x direction, radius in y direction, rotation of the axis, large arc flag, sweep flag and the absolute end position.
    ArcAbs(T, T, T, bool, bool, (T, T)),
    /// Draws an arc with radius in x direction, radius in y direction, rotation of the axis, large arc flag, sweep flag and the end position relative current to the current position.
    ArcRel(T, T, T, bool, bool, (T, T)),
    /// Draws a horizontal line to an absolute x position.
    LinetoHorizontalAbs(T),
    /// Draws a horizontal line with a certain length.
    LinetoHorizontalRel(T),
    /// Draws a vertical line to an absolute y position.
    LinetoVerticalAbs(T),
    /// Draws a vertical line with a certain length.
    LinetoVerticalRel(T),
    /// Draws a cubic Bézier curve with a calculated first control point and absolute positions for the second control point and end position.
    CurvetoCubicSmoothAbs((T, T), (T, T)),
    /// Draws a cubic Bézier curve with a calculated first control point and relative positions for the second control point and end position.
    CurvetoCubicSmoothRel((T, T), (T, T)),
    /// Draws a quadratic Bézier curve with a calculated control point and the absolute end position.
    CurvetoQuadraticSmoothAbs((T, T)),
    /// Draws a quadratic Bézier curve with a calculated control point and the relative end position.
    CurvetoQuadraticSmoothRel((T, T)),
}

impl<T> From<Primitive<T>> for PathSeg<T> {
    fn from(primitive: Primitive<T>) -> PathSeg<T> {
        match primitive {
            Primitive::Closepath => PathSeg::Closepath,
            Primitive::Moveto(p) => PathSeg::MovetoAbs(p),
            Primitive::Lineto(p) => PathSeg::LinetoAbs(p),
            Primitive::CurvetoCubic(p1, p2, p) => PathSeg::CurvetoCubicAbs(p1, p2, p),
            Primitive::CurvetoQuadratic(p1, p) => PathSeg::CurvetoQuadraticAbs(p1, p),
            Primitive::Arc(r1, r2, rotation, large_arc_flag, sweep_flag, p) =>
                PathSeg::ArcAbs(r1, r2, rotation, large_arc_flag, sweep_flag, p)
        }
    }
}

impl<T> PathSeg<T> {
    fn path_seg_type(self) -> PathSegType {
        match self {
            PathSeg::Closepath => PathSegType::ClosepathRel,
            PathSeg::MovetoAbs( .. ) => PathSegType::MovetoAbs,
            PathSeg::MovetoRel( .. ) => PathSegType::MovetoRel,
            PathSeg::LinetoAbs( .. ) => PathSegType::LinetoAbs,
            PathSeg::LinetoRel( .. ) => PathSegType::LinetoRel,
            PathSeg::CurvetoCubicAbs( .. ) => PathSegType::CurvetoCubicAbs,
            PathSeg::CurvetoCubicRel( .. ) => PathSegType::CurvetoCubicRel,
            PathSeg::CurvetoQuadraticAbs( .. ) => PathSegType::CurvetoQuadraticAbs,
            PathSeg::CurvetoQuadraticRel( .. ) => PathSegType::CurvetoQuadraticRel,
            PathSeg::ArcAbs( .. ) => PathSegType::ArcAbs,
            PathSeg::ArcRel( .. ) => PathSegType::ArcRel,
            PathSeg::LinetoHorizontalAbs( .. ) => PathSegType::LinetoHorizontalAbs,
            PathSeg::LinetoHorizontalRel( .. ) => PathSegType::LinetoHorizontalRel,
            PathSeg::LinetoVerticalAbs( .. ) => PathSegType::LinetoVerticalAbs,
            PathSeg::LinetoVerticalRel( .. ) => PathSegType::LinetoVerticalRel,
            PathSeg::CurvetoCubicSmoothAbs( .. ) => PathSegType::CurvetoCubicSmoothAbs,
            PathSeg::CurvetoCubicSmoothRel( .. ) => PathSegType::CurvetoCubicSmoothRel,
            PathSeg::CurvetoQuadraticSmoothAbs( .. ) => PathSegType::CurvetoQuadraticSmoothAbs,
            PathSeg::CurvetoQuadraticSmoothRel( .. ) => PathSegType::CurvetoQuadraticSmoothRel,
        }
    }
}

impl<T: Display + Copy> fmt::Debug for PathSeg<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut writer = PathSegWriter::new(f, true, None);
        let seg = self.clone();
        writer.write(seg)
    }
}

/// Parses one `PathSeg` at a time.
///
/// `&mut PathSegReader` implements an `Iterator` that parses one `PathSeg` each time `next()` is called.
/// Using the `Iterator` allows parsing and further processing without needing to allocate memory
/// on the heap.
///
/// # Failures
///
/// 
/// The SVG standard mandates that everything up to an error is processed, but it still would be
/// nice to have some warnings.
///
/// # Examples
///
/// ```
/// use svg_util::path::PathSegReader;
///
/// let mut parser : PathSegReader<'static, f64> = PathSegReader::new("M 0 0 h 1 v 1 h -1 z");
/// 
/// for token in parser {
///     match token {
///         Ok(segment) => println!("{:?}", segment),
///         Err(error) => {}
///     }
/// }
/// ```
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct PathSegReader<'a, T> {
    /// The rest of the string to be parsed.
    src: &'a [u8],
    /// The type of the last command that was parsed.
    /// This is needed because when the Type doesn't change it doesn't need to be repeated.
    /// E.g. `V 0 1` is equal to `V 0 V 1`
    mode: Option<PathSegType>,
    /// The first PathSeg must be a move. If this bool is set a non-move will lead to an error.
    first: bool,
    /// Needed to implement the Iterator.
    phantom: PhantomData<T>,
    /// The maximum precision that occured up to now.
    max_precision: usize,
}

// TODO: Add more info like where in the string the error happened.
// (begin of `PathSeg` + begin of the Error)
/// Errors that can occur while parsing.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Error {
    /// Nothing left in the string.
    /// This can occur when using `from_str`,
    EndOfString,
    /// Expected the mode to be set, but got another character.
    ExpectedModeCharacter(u8),
    /// Paths need to start with a Moveto command, but this one didn't.
    ExpectedMove,
    /// Expected a number but got something else.
    ExpectedNumber,
    /// Expected a flag, `0` or `1`.
    ExpectedFlag,
    /// A correct number was found, but it was not possible to convert it into the number type used.
    NumberParseError,
}

// FIXME: maybe an own error type makes more sense?
impl<T: FromStr + Copy> FromStr for PathSeg<T> {
    type Err = Error;
    fn from_str(s: &str) -> Result<PathSeg<T>, Error> {
        let mut reader = PathSegReader::new_arbitrary(s);
        match reader.pop_one_pathseg() {
            Some(result) => result,
            None => Err(Error::EndOfString)
        }
        // FIXME: the rest of the string needs to be empty!
    }
}

impl<'a, T: Copy + FromStr> PathSegReader<'a, T> {
    /// Creates a new `PathSegReader` using for instance a `&str` or a `&[u8]`.
    pub fn new<B: AsRef<[u8]> + ?Sized>(src: &'a B) -> PathSegReader<'a, T> {
        PathSegReader {
            src: src.as_ref(),
            mode: None,
            first: true,
            phantom: PhantomData,
            max_precision: 0,
        }
    }
    // In contrast to the reader created by `new` this allows parsing arbitrary
    // `PathSeg` sequences, that don't have to start with move `PathSeg`s.
    fn new_arbitrary(src: &'a str) -> PathSegReader<'a, T> {
        PathSegReader {
            src: src.as_ref(),
            mode: None,
            first: false,
            phantom: PhantomData,
            max_precision: 0,
        }
    }

    /// Parses the first `PathSeg` and removes it from the source.
    ///
    /// Returns `None` when the source doesn't contain any `PathSeg`s
    /// anymore (empty or only whitespace).
    /// Otherwise it returns either the `PathSeg` or an `Error` if one
    /// occured.
    pub fn pop_one_pathseg(&mut self) -> Option<Result<PathSeg<T>, Error>> {
        self.remove_leading_whitespace();
        if self.src.is_empty() {
            None
        } else {
            Some(self.pop_empty())
        }
    }

    /// Returns the maximum precision encountered so far.
    pub fn precision(&self) -> usize {
        self.max_precision
    }

    // Pop one `PathSeg` with the precondition that there are
    // `PathSegs` left.
    fn pop_empty(&mut self) -> Result<PathSeg<T>, Error> {

        // Look ahead if the next char changes the type of the `PathSeg`
        let first = match self.src.first() {
            None => {
                panic!("Path string shouldn't be empty but it is!");
            }
            Some(c) => *c,
        };
        if let Some(mode) = PathSegType::from_u8(first) {
            self.mode = Some(mode);
            self.src = &self.src[1..];
            self.remove_leading_whitespace();
        } else {
            // there might be a comma seperating path segments if there's no
            // letter. E.g.: `h 1, 1` is equal to `h 1 h 1`
            self.remove_leading_comma_whitespace();
        }

        let mode = match self.mode {
            Some(mode) => mode,
            None => return Err(Error::ExpectedModeCharacter(first)),
        };

        // First Path Segment needs to be a Moveto
        if self.first {
            match mode {
                PathSegType::MovetoAbs | PathSegType::MovetoRel => {}
                _ => {
                    return Err(Error::ExpectedMove);
                }
            }
            self.first = false;
        }

        let ret = match mode {
            PathSegType::ClosepathAbs | PathSegType::ClosepathRel => {
                // `Closepath` can't be repeated without reusing 'z'/'Z'.
                self.mode = None;
                Ok(PathSeg::Closepath)
            }
            PathSegType::MovetoAbs => {
                Ok(PathSeg::MovetoAbs(try!(self.get_coordinate_pair())))
            }
            PathSegType::MovetoRel => {
                Ok(PathSeg::MovetoRel(try!(self.get_coordinate_pair())))
            }
            PathSegType::LinetoAbs => {
                Ok(PathSeg::LinetoAbs(try!(self.get_coordinate_pair())))
            }
            PathSegType::LinetoRel => {
                Ok(PathSeg::LinetoRel(try!(self.get_coordinate_pair())))
            }
            PathSegType::CurvetoCubicAbs => {
                let (p1, p2, p) = try!(self.get_three_coordinate_pairs());
                Ok(PathSeg::CurvetoCubicAbs(p1, p2, p))
            }
            PathSegType::CurvetoCubicRel => {
                let (p1, p2, p) = try!(self.get_three_coordinate_pairs());
                Ok(PathSeg::CurvetoCubicRel(p1, p2, p))
            }
            PathSegType::CurvetoQuadraticAbs => {
                let (p1, p) = try!(self.get_two_coordinate_pairs());
                Ok(PathSeg::CurvetoQuadraticAbs(p1, p))
            }
            PathSegType::CurvetoQuadraticRel => {
                let (p1, p) = try!(self.get_two_coordinate_pairs());
                Ok(PathSeg::CurvetoQuadraticRel(p1, p))
            }
            PathSegType::ArcAbs => {
                let (r1, r2, rotation, large_arc_flag, sweep_flag, p) =
                    try!(self.get_arc_argument());
                Ok(PathSeg::ArcAbs(r1, r2, rotation, large_arc_flag, sweep_flag, p))
            }
            PathSegType::ArcRel => {
                let (r1, r2, rotation, large_arc_flag, sweep_flag, p) =
                    try!(self.get_arc_argument());
                Ok(PathSeg::ArcRel(r1, r2, rotation, large_arc_flag, sweep_flag, p))
            }
            PathSegType::LinetoHorizontalAbs => {
                Ok(PathSeg::LinetoHorizontalAbs(try!(self.get_number(false))))
            }
            PathSegType::LinetoHorizontalRel => {
                Ok(PathSeg::LinetoHorizontalRel(try!(self.get_number(false))))
            }
            PathSegType::LinetoVerticalAbs => {
                Ok(PathSeg::LinetoVerticalAbs(try!(self.get_number(false))))
            }
            PathSegType::LinetoVerticalRel => {
                Ok(PathSeg::LinetoVerticalRel(try!(self.get_number(false))))
            }
            PathSegType::CurvetoCubicSmoothAbs => {
                let (p2, p) = try!(self.get_two_coordinate_pairs());
                Ok((PathSeg::CurvetoCubicSmoothAbs(p2, p)))
            }
            PathSegType::CurvetoCubicSmoothRel => {
                let (p2, p) = try!(self.get_two_coordinate_pairs());
                Ok((PathSeg::CurvetoCubicSmoothRel(p2, p)))
            }
            PathSegType::CurvetoQuadraticSmoothAbs => {
                Ok(PathSeg::CurvetoQuadraticSmoothAbs(try!(self.get_coordinate_pair())))
            }
            PathSegType::CurvetoQuadraticSmoothRel => {
                Ok(PathSeg::CurvetoQuadraticSmoothRel(try!(self.get_coordinate_pair())))
            }
        };
        return ret;
    }

    fn remove_leading_whitespace(&mut self) {
        while let Some(c) = self.src.first() {
            if [0x20, 0x9, 0xD, 0xA].contains(c) {
                self.src = &self.src[1..];
            } else {
                return;
            }
        }
    }

    fn remove_leading_comma_whitespace(&mut self) {
        self.remove_leading_whitespace();
        if let Some(&b',') = self.src.first() {
            self.src = &self.src[1..];
            self.remove_leading_whitespace();
        }
    }

    fn get_number(&mut self, nonnegative: bool) -> Result<T, Error> {
        let mut length = 0;
        let mut precision : usize = 0;

        if !nonnegative {
            match self.src.get(length) {
                Some(&b'+') | Some(&b'-') => {
                    length += 1;
                }
                _ => {}
            }
        }

        while let Some(c) = self.src.get(length) {
            match *c {
                b'0'...b'9' => {
                    length += 1;
                }
                _ => break,
            }
        }
        if let Some(&b'.') = self.src.get(length) {
            length += 1;
            while let Some(c) = self.src.get(length) {
                match *c {
                    b'0'...b'9' => {
                        length += 1;
                        precision += 1;
                    }
                    _ => break,
                }
            }
            // "." is not a valid svg number.
            if length < 2 {
                return Err(Error::ExpectedNumber);
            }
        } else if length == 0 {
            // An empty string is not a valid svg number.
            return Err(Error::ExpectedNumber);
        }

        match self.src.get(length) {
            Some(&b'e') | Some(&b'E') => {
                length += 1;
                let negative = match self.src.get(length) {
                    Some(&b'+')  => {
                        length += 1;
                        false
                    }
                    Some(&b'-') => {
                        length += 1;
                        true
                    }
                    _ => false
                };
                let mut length_after_e = length;
                while let Some(c) = self.src.get(length_after_e) {
                    match *c {
                        b'0'...b'9' => {
                            length_after_e += 1;
                        }
                        _ => break,
                    }
                }
                if length_after_e == length {
                    // Digits need to follow after an `e`/`E`.
                    return Err(Error::ExpectedNumber);
                }
                // Parse the number after `e` into a `usize` and subtract it from the precision.
                // E.g. 0.1e1 == 1 with precision 0.
                let num = &self.src[length .. length_after_e];
                let numstring = unsafe { from_utf8_unchecked(num) };
                match usize::from_str(numstring) {
                    Ok(num) => {
                        if negative {
                            precision += num;
                        } else {
                            precision = precision.saturating_sub(num);
                        }
                    }
                    _ => {}
                }
                length = length_after_e;
            }
            _ => {}
        }

        if precision > self.max_precision {
            self.max_precision = precision;
        }

        let (num, rest) = self.src.split_at(length);
        self.src = rest;

        // Safe because we just matched only numbers, '+', '-', '.', 'e' and 'E's.
        let numstring = unsafe { from_utf8_unchecked(num) };
        match T::from_str(numstring) {
            Ok(num) => Ok(num),
            _ => Err(Error::NumberParseError),
        }
    }

    fn get_coordinate_pair(&mut self) -> Result<(T, T), Error> {
        let x = try!(self.get_number(false));
        self.remove_leading_comma_whitespace();
        let y = try!(self.get_number(false));
        Ok((x, y))
    }

    fn get_two_coordinate_pairs(&mut self) -> Result<((T, T), (T, T)), Error> {
        let p1 = try!(self.get_coordinate_pair());
        self.remove_leading_comma_whitespace();
        let p2 = try!(self.get_coordinate_pair());
        Ok((p1, p2))
    }

    fn get_three_coordinate_pairs(&mut self) -> Result<((T, T), (T, T), (T, T)), Error> {
        let p1 = try!(self.get_coordinate_pair());
        self.remove_leading_comma_whitespace();
        let p2 = try!(self.get_coordinate_pair());
        self.remove_leading_comma_whitespace();
        let p3 = try!(self.get_coordinate_pair());
        Ok((p1, p2, p3))
    }

    fn get_arc_argument(&mut self) -> Result<(T, T, T, bool, bool, (T, T)), Error> {
        let (r1, r2) = try!(self.get_coordinate_pair());
        self.remove_leading_comma_whitespace();
        let rotation = try!(self.get_number(true));
        self.remove_leading_comma_whitespace();

        let large_arc_flag = match self.src.first() {
            Some(&b'1') => true,
            Some(&b'0') => false,
            _ => return Err(Error::ExpectedFlag),
        };
        self.src = &self.src[1..];

        self.remove_leading_comma_whitespace();

        let sweep_flag = match self.src.first() {
            Some(&b'1') => true,
            Some(&b'0') => false,
            _ => return Err(Error::ExpectedFlag),
        };
        self.src = &self.src[1..];

        self.remove_leading_comma_whitespace();

        let p = try!(self.get_coordinate_pair());
        Ok((r1, r2, rotation, large_arc_flag, sweep_flag, p))
    }
}

/// Parse all `PathSeg`s and put them in a newly allocated array.
///
/// Returns all `PathSeg`s that were parsed until an error occurred or the string was empty,
/// the error if one occured and the maximum precision.
pub fn parse_all_pathsegs<'a, T: Copy + FromStr, B: AsRef<[u8]> + ?Sized>(src: &'a B) -> (Vec<PathSeg<T>>, Option<Error>, usize) {
    let mut parser = PathSegReader::new(src);
    let mut array = Vec::new();
    loop {
        let result = match parser.pop_one_pathseg() {
            Some(result) => result,
            None => break
        };
        match result {
            Ok(seg) => { array.push(seg) }
            Err(e) => { return (array, Some(e), parser.precision()) }
        }
        
    }
    (array, None, parser.precision())
}

/// The `Iterator` for `PathSegReader`.
pub struct Segs<'a, T> {
    reader: PathSegReader<'a, T>,
    done: bool,
}

impl<'a, T: Copy + FromStr> Iterator for Segs<'a, T> {
    type Item = Result<PathSeg<T>, Error>;

    fn next(&mut self) -> Option<Result<PathSeg<T>, Error>> {
        if self.done {
            return None;
        }

        let next = self.reader.pop_one_pathseg();

        // Don't emit anything after an error occured.
        if let Some(Err(_)) = next {
            self.done = true;
        }
        
        next
    }
}

impl<'a, T> From<PathSegReader<'a, T>> for Segs<'a, T> {
    fn from(reader: PathSegReader<'a, T>) -> Segs<'a, T> {
        Segs {
            reader: reader,
            done: false,
        }
    }
}

impl<'a, T: Copy + FromStr> IntoIterator for PathSegReader<'a, T> {
    type Item = Result<PathSeg<T>, Error>;
    type IntoIter = Segs<'a, T>;

    fn into_iter(self) -> Segs<'a, T> {
        self.into()
    }
}

/// Writes `PathSeg`s as strings either in a space-saving or human readable format.
///
/// Because this needs a mutable `Write` you first have to
/// let the `PathSegWriter` let go out of scope again before
/// using the `Write` again.
/// FIXME: example
pub struct PathSegWriter<'a, W: 'a + Write> {
    /// Where to write to.
    sink: &'a mut W,
    /// The type of the last written `PathSeg`.
    mode: Option<PathSegType>,
    /// Wheter to pretty-print or have an optimized output.
    pretty: bool,
    /// Limit amount of digits after the decimal to print.
    precision: Option<usize>,
    /// What kind the token was that was last written.
    last_token: TokenWritten,
}

impl<'a, W: 'a + Write> PathSegWriter<'a, W> {
    /// Creates a new `PathSegWriter`.
    ///
    /// The `pretty` argument decides wether to write in a space-saving or pretty, human-readable way.
    /// `precision` sets the maximum amount of digits printed after a decimal. `None` means no limit.
    /// Warning: If you have many small relative segments, the error can add up and distort the path dramatically. 
    pub fn new(sink: &'a mut W, pretty: bool, precision: Option<usize>) -> PathSegWriter<'a, W> {
        PathSegWriter {
            sink: sink,
            mode: None,
            pretty: pretty,
            precision: precision,
            last_token: TokenWritten::NotANumber,
        }
    }
    
    /// Write one number
    fn write_num<T: Display>(&mut self, num: T) -> Result<(), fmt::Error> {
        if self.pretty {
            // Always write a space when pretty printing.
            try!(self.sink.write_char(' '));
            if let Some(precision) = self.precision {
                let mut filter = DropTrailingZeros::new(&mut self.sink);
                try!(write!(filter, "{:.*}", precision, num));
            } else {
                try!(write!(self.sink, "{}", num));
            }
            return Ok(());
        } else {
            let mut optimized_writer = DropLeadingZero::new(&mut self.sink, self.last_token);
            if let Some(precision) = self.precision {
                let mut filter = DropTrailingZeros::new(&mut optimized_writer);
                try!(write!(filter, "{:.*}", precision, num));
            } else {
                try!(write!(optimized_writer, "{}", num));
            }
            self.last_token = try!(optimized_writer.finish_and_return_token_written());
        }
        Ok(())
    }
    
    /// Write a x, y pair of numbers.
    fn write_pair<T: Display>(&mut self, pair: (T,T)) -> Result<(), fmt::Error> {
        let (x,y) = pair;
        try!(self.write_num(x));
        self.write_num(y)
    }
    
    /// Write a flag (used in arcs).
    fn write_flag(&mut self, flag: bool) -> Result<(), fmt::Error> {
        if self.pretty || self.last_token != TokenWritten::NotANumber {
            try!(self.sink.write_char(' '));
        }
        try!(self.sink.write_char(if flag { '1' } else { '0' }));
        self.last_token = TokenWritten::NotANumber;
        Ok(())
    }

    /// Write a `PathSeg`.
    pub fn write<T: Display + Copy>(&mut self, path_seg: PathSeg<T>) -> Result<(), fmt::Error> {
        let old_mode = self.mode;
        let path_seg_type = path_seg.path_seg_type();
        self.mode = Some(path_seg_type);

        if self.pretty {
            match path_seg_type {
                // Moves should stand on new lines when pretty printing.
                PathSegType::MovetoAbs | PathSegType::MovetoRel => {
                    try!(self.sink.write_char('\n'));
                }
                // Other `PathSeg`s should be seperated by spaces.
                _ => {
                    try!(self.sink.write_char(' '));
                }
            }
        }

        // Don't repeat the character for setting the type of the `PathSeg` when it's still the same.
        // Except: `z`/`Z` or when pretty printing.
        let need_mode_character = self.pretty || match (path_seg_type, old_mode) {
            (_, None) => true,
            (PathSegType::ClosepathAbs, _) => true,
            (PathSegType::ClosepathRel, _) => true,
            (PathSegType::LinetoAbs, Some(PathSegType::MovetoAbs)) => false,
            (PathSegType::LinetoRel, Some(PathSegType::MovetoRel)) => false,
            (new, Some(old)) => new != old           
        };

        if need_mode_character {
            try!(self.sink.write_char(path_seg_type.into()));
            self.last_token = TokenWritten::NotANumber;
        }

        match path_seg {
            PathSeg::Closepath => Ok(()),
            PathSeg::MovetoAbs(p) |
            PathSeg::MovetoRel(p) |
            PathSeg::LinetoAbs(p) |
            PathSeg::LinetoRel(p) =>
                self.write_pair(p),
            PathSeg::CurvetoCubicAbs(p1, p2, p) |
            PathSeg::CurvetoCubicRel(p1, p2, p) => {
                try!(self.write_pair(p1));
                try!(self.write_pair(p2));
                self.write_pair(p)
            }
            PathSeg::CurvetoQuadraticAbs(p1, p) |
            PathSeg::CurvetoQuadraticRel(p1, p) => {
                try!(self.write_pair(p1));
                self.write_pair(p)
            }
            PathSeg::ArcAbs(r1, r2, rotation, large_arc_flag, sweep_flag, p) |
            PathSeg::ArcRel(r1, r2, rotation, large_arc_flag, sweep_flag, p) => {
                try!(self.write_num(r1));
                try!(self.write_num(r2));
                try!(self.write_num(rotation));
                try!(self.write_flag(large_arc_flag));
                try!(self.write_flag(sweep_flag));
                self.write_pair(p)
            }
            PathSeg::LinetoHorizontalAbs(x) |
            PathSeg::LinetoHorizontalRel(x) =>
                self.write_num(x),
            PathSeg::LinetoVerticalAbs(y) |
            PathSeg::LinetoVerticalRel(y) =>
                self.write_num(y),
            PathSeg::CurvetoCubicSmoothAbs(p2, p) |
            PathSeg::CurvetoCubicSmoothRel(p2, p) => {
                try!(self.write_pair(p2));
                self.write_pair(p)
            }
            PathSeg::CurvetoQuadraticSmoothAbs(p) |
            PathSeg::CurvetoQuadraticSmoothRel(p) =>
                self.write_pair(p),
        }
    }

    /// Test how much bytes writing a `PathSeg` would emit, without changing any state.
    pub fn test_write<T: Display + Copy>(&self, path_seg: PathSeg<T>) -> usize {
        let mut count = Count { len: 0 };
        let mut psw_copy = PathSegWriter {
            sink: &mut count,
            mode: self.mode,
            pretty: self.pretty,
            precision: self.precision,
            last_token: self.last_token,
        };
        let res = psw_copy.write(path_seg);
        if res.is_err() {
            panic!("writing in a `Count` shouldn't return Errors");
        }
        psw_copy.sink.len
    }
}

/// Writes all `PathSeg`s from a slice into a `Write`.
///
/// # Examples
/// 
/// ```
/// use svg_util::path::{PathSeg, write_all_pathsegs};
///
/// let mut str = String::new();
/// let segs : [PathSeg<i8>; 2] = [PathSeg::MovetoAbs((1,1)), PathSeg::LinetoRel((1,1))];
/// write_all_pathsegs(&mut str, &segs, false, None).unwrap();
/// assert_eq!(str, "M1 1l1 1");
/// ```
pub fn write_all_pathsegs<'a, W: Write, T: Display + Copy>(sink: &mut W, pathsegs: &'a[PathSeg<T>], pretty: bool, precision: Option<usize>) -> Result<(), fmt::Error> {
    let mut writer = PathSegWriter::new(sink, pretty, precision);
    for seg in pathsegs {
        try!(writer.write(seg.clone()));
    }
    Ok(())
}

/// Converts `PathSeg`s to `Primitive`s.
pub struct PathSegToPrimitive<T> {
    /// The position we're currently at.
    pos: (T, T),
    /// Where we moved to with the last move.
    last_move: (T, T),
    /// The position we're going to predict for a cubic smooth `PathSeg`.
    cubic_smooth: (T, T),
    /// The position we're going to predict for a quadratic smooth `PathSeg`.
    quadratic_smooth: (T, T),
}

impl <T: Default> PathSegToPrimitive<T> {
    /// Create a new `PathSegToPrimitive` converter.
    /// It saves the state necessary to transform relative, smooth, and vertical/horizontal Segments into `Primitive`s
    ///
    /// FIXME: Doesn't work correctly for paths starting with `m`, if the `Default` value isn't zero.
    /// The `Zero` trait however is still marked as unstable.
    pub fn new() -> Self {
        PathSegToPrimitive {
            pos: (Default::default(), Default::default()),
            last_move: (Default::default(), Default::default()),
            cubic_smooth: (Default::default(), Default::default()),
            quadratic_smooth: (Default::default(), Default::default()),
        }
    }
}

fn to_abs<T: Add<T, Output=T>>(pos: (T,T), rel: (T, T)) -> (T, T) {
    let (rel_x, rel_y) = rel;
    let (pos_x, pos_y) = pos;
    (pos_x + rel_x, pos_y + rel_y)
}

fn predict<T: Copy + Add<T, Output=T> + Sub<T, Output=T>>(new_pos: (T,T), point: (T, T)) -> (T, T) {
    let (point_x, point_y) = point;
    let (pos_x, pos_y) = new_pos;
    let (diff_x, diff_y) = (pos_x - point_x, pos_y - point_y);
    (pos_x + diff_x, pos_y + diff_y)
}

impl <T: Copy + Add<T, Output=T> + Sub<T, Output=T>> PathSegToPrimitive<T> {
    /// Convert a `PathSeg` to a `Primitive`.
    pub fn convert(&mut self, seg: PathSeg<T>) -> Primitive<T> {
        match seg {
            PathSeg::Closepath => {
                self.pos = self.last_move;
                self.cubic_smooth = self.pos;
                self.quadratic_smooth = self.pos;
                Primitive::Closepath
            }
            PathSeg::MovetoAbs(p) => {
                self.pos = p;
                self.cubic_smooth = p;
                self.quadratic_smooth = p;
                self.last_move = p;
                Primitive::Moveto(p)
            }
            PathSeg::MovetoRel(p) => {
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = p_abs;
                self.quadratic_smooth = p_abs;
                self.last_move = p_abs;
                Primitive::Moveto(p_abs)
            }
            PathSeg::LinetoAbs(p) => {
                self.pos = p;
                self.cubic_smooth = p;
                self.quadratic_smooth = p;
                Primitive::Lineto(p)
            }
            PathSeg::LinetoRel(p) => {
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = p_abs;
                self.quadratic_smooth = p_abs;
                Primitive::Lineto(p_abs)
            }
            PathSeg::CurvetoCubicAbs(p1, p2, p) => {
                self.pos = p;
                self.cubic_smooth = predict(p, p2);
                self.quadratic_smooth = p;
                Primitive::CurvetoCubic(p1, p2, p)
            }
            PathSeg::CurvetoCubicRel(p1, p2, p) => {
                let p1_abs = to_abs(self.pos, p1);
                let p2_abs = to_abs(self.pos, p2);
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = predict(p_abs, p2_abs);
                self.quadratic_smooth = p_abs;
                Primitive::CurvetoCubic(p1_abs, p2_abs, p_abs)
            }
            PathSeg::CurvetoQuadraticAbs(p1, p) => {
                self.pos = p;
                self.cubic_smooth = p;
                self.quadratic_smooth = predict(p, p1);
                Primitive::CurvetoQuadratic(p1, p)
            }
            PathSeg::CurvetoQuadraticRel(p1, p) => {
                let p1_abs = to_abs(self.pos, p1);
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = p_abs;
                self.quadratic_smooth = predict(p_abs, p1_abs);
                Primitive::CurvetoQuadratic(p1_abs, p_abs)
            }
            PathSeg::ArcAbs(r1, r2, rotation, large_arc_flag, sweep_flag, p) => {
                self.pos = p;
                self.cubic_smooth = p;
                self.quadratic_smooth = p;
                Primitive::Arc(r1, r2, rotation, large_arc_flag, sweep_flag, p)
            }
            PathSeg::ArcRel(r1, r2, rotation, large_arc_flag, sweep_flag, p) => {
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = p_abs;
                self.quadratic_smooth = p_abs;
                Primitive::Arc(r1, r2, rotation, large_arc_flag, sweep_flag, p_abs)
            }
            PathSeg::LinetoHorizontalAbs(x) => {
                 let (_, y) = self.pos;
                 let p = (x, y);
                 
                 self.pos = p;
                 self.cubic_smooth = p;
                 self.quadratic_smooth = p;
                 Primitive::Lineto(p)
            }
            PathSeg::LinetoHorizontalRel(x) => {
                 let (x_old, y) = self.pos;
                 let p = (x + x_old, y);
                 
                 self.pos = p;
                 self.cubic_smooth = p;
                 self.quadratic_smooth = p;
                 Primitive::Lineto(p)
            }
            PathSeg::LinetoVerticalAbs(y) => {
                 let (x, _) = self.pos;
                 let p = (x, y);
                 
                 self.pos = p;
                 self.cubic_smooth = p;
                 self.quadratic_smooth = p;
                 Primitive::Lineto(p)
            }
            PathSeg::LinetoVerticalRel(y) => {
                 let (x, y_old) = self.pos;
                 let p = (x, y + y_old);
                 
                 self.pos = p;
                 self.cubic_smooth = p;
                 self.quadratic_smooth = p;
                 Primitive::Lineto(p)
            }
            PathSeg::CurvetoCubicSmoothAbs(p2, p) => {
                let p1 = self.cubic_smooth;
                
                self.pos = p;
                self.cubic_smooth = predict(p, p2);
                self.quadratic_smooth = p;
                Primitive::CurvetoCubic(p1, p2, p)
            }
            PathSeg::CurvetoCubicSmoothRel(p2, p) => {
                let p1_abs = self.cubic_smooth;
                let p2_abs = to_abs(self.pos, p2);
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = predict(p_abs, p2_abs);
                self.quadratic_smooth = p_abs;
                Primitive::CurvetoCubic(p1_abs, p2_abs, p_abs)
            }
            PathSeg::CurvetoQuadraticSmoothAbs(p) => {
                let p1 = self.quadratic_smooth;
                
                self.pos = p;
                self.cubic_smooth = p;
                self.quadratic_smooth = predict(p, p1);
                Primitive::CurvetoQuadratic(p1, p)
            }
            PathSeg::CurvetoQuadraticSmoothRel(p) => {
                let p1_abs = self.quadratic_smooth;
                let p_abs = to_abs(self.pos, p);
                
                self.pos = p_abs;
                self.cubic_smooth = p_abs;
                self.quadratic_smooth = predict(p_abs, p1_abs);
                Primitive::CurvetoQuadratic(p1_abs, p_abs)
            }
        }
    }
}