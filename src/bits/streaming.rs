//! Bit level parsers
//!

use crate::error::{ErrorKind, ParseError};
use crate::internal::{Err, IResult, Needed};
use crate::lib::std::ops::{AddAssign, Div, Shl, Shr};
use crate::traits::{Input, ToUsize};

/// Generates a parser taking `count` bits
pub fn take<I, O, C, E: ParseError<(I, usize)>>(
  count: C,
) -> impl Fn((I, usize)) -> IResult<(I, usize), O, E>
where
  I: Input<Item = u8>,
  C: ToUsize,
  O: From<u8> + AddAssign + Shl<usize, Output = O> + Shr<usize, Output = O>,
{
  let count = count.to_usize();
  move |(input, bit_offset): (I, usize)| {
    if count == 0 {
      Ok(((input, bit_offset), 0u8.into()))
    } else {
      let cnt = (count + bit_offset).div(8);
      if input.input_len() * 8 < count + bit_offset {
        Err(Err::Incomplete(Needed::new(count)))
      } else {
        let mut acc: O = 0_u8.into();
        let mut offset: usize = bit_offset;
        let mut remaining: usize = count;
        let mut end_offset: usize = 0;

        for byte in input.iter_elements().take(cnt + 1) {
          if remaining == 0 {
            break;
          }
          let byte = if offset == 0 { byte } else { byte >> offset };
          let byte = if remaining < 8 {
            let shift_offset = 8 - remaining;
            // Clear upper 8-N bits so we're left with N bits
            (byte << shift_offset) >> shift_offset
          } else {
            byte
          };
          let val: O = byte.into();

          let taken_without_truncation = 8 - offset;
          acc += val << (count - remaining);
          if remaining < taken_without_truncation {
            end_offset = remaining + offset;
            break;
          } else {
            remaining -= taken_without_truncation;
            offset = 0;
          }
        }
        Ok(((input.take_from(cnt), end_offset), acc))
      }
    }
  }
}

/// Generates a parser taking `count` bits and comparing them to `pattern`
pub fn tag<I, O, C, E: ParseError<(I, usize)>>(
  pattern: O,
  count: C,
) -> impl Fn((I, usize)) -> IResult<(I, usize), O, E>
where
  I: Input<Item = u8> + Clone,
  C: ToUsize,
  O: From<u8> + AddAssign + Shl<usize, Output = O> + Shr<usize, Output = O> + PartialEq,
{
  let count = count.to_usize();
  move |input: (I, usize)| {
    let inp = input.clone();

    take(count)(input).and_then(|(i, o)| {
      if pattern == o {
        Ok((i, o))
      } else {
        Err(Err::Error(error_position!(inp, ErrorKind::TagBits)))
      }
    })
  }
}

/// Parses one specific bit as a bool.
///
/// # Example
/// ```rust
/// # use nom::bits::streaming::bool;
/// # use nom::IResult;
/// # use nom::error::{Error, ErrorKind};
///
/// fn parse(input: (&[u8], usize)) -> IResult<(&[u8], usize), bool> {
///     bool(input)
/// }
///
/// assert_eq!(parse(([0b00000001].as_ref(), 0)), Ok((([0b00000001].as_ref(), 1), true)));
/// assert_eq!(parse(([0b00000001].as_ref(), 1)), Ok((([0b00000001].as_ref(), 2), false)));
/// ```
pub fn bool<I, E: ParseError<(I, usize)>>(input: (I, usize)) -> IResult<(I, usize), bool, E>
where
  I: Input<Item = u8>,
{
  let (res, bit): (_, u32) = take(1usize)(input)?;
  Ok((res, bit != 0))
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_take_0() {
    let input = [0b0010_0001].as_ref();

    let result: crate::IResult<(&[u8], usize), usize> = take(0usize)((input, 0));

    assert_eq!(result, Ok(((input, 0), 0)));
  }

  #[test]
  fn test_tag_ok() {
    let input = [0b11110001].as_ref();
    let offset = 0usize;
    let bits_to_take = 4usize;
    let value_to_tag = 0b0001;

    let result: crate::IResult<(&[u8], usize), usize> =
      tag(value_to_tag, bits_to_take)((input, offset));

    assert_eq!(result, Ok(((input, bits_to_take), value_to_tag)));
  }

  #[test]
  fn test_tag_err() {
    let input = [0b11110001].as_ref();
    let offset = 0usize;
    let bits_to_take = 4usize;
    let value_to_tag = 0b1111;

    let result: crate::IResult<(&[u8], usize), usize> =
      tag(value_to_tag, bits_to_take)((input, offset));

    assert_eq!(
      result,
      Err(crate::Err::Error(crate::error::Error {
        input: (input, offset),
        code: ErrorKind::TagBits
      }))
    );
  }

  #[test]
  fn test_bool_0() {
    let input = [0b00000001].as_ref();

    let result: crate::IResult<(&[u8], usize), bool> = bool((input, 0));

    assert_eq!(result, Ok(((input, 1), true)));
  }

  #[test]
  fn test_bool_eof() {
    let input = [0b00000001].as_ref();

    let result: crate::IResult<(&[u8], usize), bool> = bool((input, 8));

    assert_eq!(result, Err(crate::Err::Incomplete(Needed::new(1))));
  }
}
