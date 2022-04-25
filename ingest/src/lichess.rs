#![allow(dead_code)]

use nom::{
    bytes::complete::{take_while, take_while_m_n},
    character::complete::{alphanumeric1, char, space0},
    multi::many0,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};

#[derive(Debug)]
struct Label<'a> {
    key: &'a str,
    value: &'a str,
}

#[derive(Debug)]
struct Move<'a> {
    number: &'a str,
    dots: &'a str,
    piece: &'a str,
    labels: Vec<Label<'a>>,
}

fn parse_pgn(input: &str) -> IResult<&str, (Vec<Label>, Vec<Move>, &str)> {
    let (input, _) = many0(char('\n'))(input)?;
    let (input, labels) = parse_labels(input)?;
    let (input, _) = many0(char('\n'))(input)?;
    let (input, moves) = parse_moves(input)?;
    let (input, result) = result(input)?;
    let (input, _) = many0(char('\n'))(input)?;

    Ok((input, (labels, moves, result)))
}

fn parse_labels(input: &str) -> IResult<&str, Vec<Label>> {
    many0(parse_label)(input)
}

fn parse_label(input: &str) -> IResult<&str, Label> {
    let (input, (key, value)) = terminated(
        delimited(char('['), tuple((label_key, label_value)), char(']')),
        char('\n'),
    )(input)?;
    Ok((input, Label { key, value }))
}

fn label_key(input: &str) -> IResult<&str, &str> {
    terminated(take_while(|c: char| c != ' '), char(' '))(input)
}

fn label_value(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), take_while(|c: char| c != '"'), char('"'))(input)
}

fn parse_moves(input: &str) -> IResult<&str, Vec<Move>> {
    many0(parse_move)(input)
}

fn parse_move(input: &str) -> IResult<&str, Move> {
    let (input, (number, dots, piece, labels)) =
        tuple((move_number, move_dots, move_piece, move_labels))(input)?;

    Ok((
        input,
        Move {
            number,
            dots,
            piece,
            labels,
        },
    ))
}

fn move_number(input: &str) -> IResult<&str, &str> {
    preceded(take_while(|c: char| c == ' ' || c == '\n'), alphanumeric1)(input)
}

fn move_dots(input: &str) -> IResult<&str, &str> {
    terminated(take_while_m_n(1, 3, |c: char| c == '.'), space0)(input)
}

fn move_piece(input: &str) -> IResult<&str, &str> {
    delimited(space0, take_while(|c: char| c != ' '), space0)(input)
}

fn move_labels(input: &str) -> IResult<&str, Vec<Label>> {
    delimited(
        space0,
        delimited(char('{'), many0(move_label), char('}')),
        space0,
    )(input)
}

fn move_label(input: &str) -> IResult<&str, Label> {
    let (input, (key, value)) = delimited(
        space0,
        delimited(
            char('['),
            tuple((move_label_key, move_label_value)),
            char(']'),
        ),
        space0,
    )(input)?;
    Ok((input, Label { key, value }))
}

fn move_label_key(input: &str) -> IResult<&str, &str> {
    preceded(char('%'), take_while(|c: char| c != ' '))(input)
}

fn move_label_value(input: &str) -> IResult<&str, &str> {
    preceded(char(' '), take_while(|c: char| c != ']'))(input)
}

fn result(input: &str) -> IResult<&str, &str> {
    preceded(space0, take_while(|c: char| c != '\n'))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_sample_with_clock_and_eval() {
        let text = include_str!("../tests/data/lichess-sample-clock-eval.pgn");
        let (remaining, (labels, moves, _)) = dbg!(parse_pgn(text).unwrap());
        assert_eq!(remaining.len(), 0);
        assert_eq!(labels.len(), 18);
        assert_eq!(moves.len(), 26);
    }

    #[test]
    pub fn test_march_2022_with_clock() {
        let text = include_str!("../tests/data/lichess-march-2022-clock.pgn");
        let (remaining, (labels, moves, _)) = parse_pgn(text).unwrap();
        assert_eq!(remaining.len(), 0);
        assert_eq!(labels.len(), 17);
        assert_eq!(moves.len(), 73);
    }
}
