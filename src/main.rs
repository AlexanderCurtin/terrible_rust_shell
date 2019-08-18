use std::env::{var, VarError};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process::*;

extern crate regex;

#[macro_use]
extern crate pest_derive;

extern crate pest;
use pest::iterators::Pair;
use pest::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ShellParser;

pub enum ShellCommand {
    InternalCommand(InternalCommand),
    ProgramName(String),
}

pub enum InternalCommand {
    Exit,
}

const PROMPT: &str = "$";
fn main() {
    prompt();
    for line in io::stdin().lock().lines() {
        execute(&line.unwrap());
        prompt();
    }
}

fn prompt() {
    print!("{} ", PROMPT);
    io::stdout().flush().unwrap();
}

fn execute(line: &String) {
    let (parsed_args, input, output) = parse(line);
    let command = match parsed_args.first() {
        Some(x) => x,
        None => return,
    };
    let parsed_command = parse_command(command);

    match parsed_command {
        ShellCommand::ProgramName(path) => execute_program(path, parsed_args, input, output),
        ShellCommand::InternalCommand(ic) => execute_internal_program(ic),
    }
}

fn execute_program(path: String, args: Vec<String>, input: Option<Stdio>, output: Option<Stdio> ) {
    let stdin = input.unwrap_or_else(|| Stdio::inherit());
    let stdout = output.unwrap_or_else(|| Stdio::inherit());

    match std::process::Command::new(path)
        .args(args.iter().skip(1))
        .stdout(stdout)
        .stdin(stdin)
        .spawn()
        .as_mut()
    {
        Err(e) => eprintln!("{:?}", e),
        Ok(c) => {
            c.wait().expect("I mean really.");
            io::stdout().flush().unwrap();
        }
    }
}

fn execute_internal_program(command: InternalCommand) {
    match command {
        InternalCommand::Exit => exit_cmd(),
    };
}

#[test]
fn test_to_strings() {
    let mut pairs = ShellParser::parse(Rule::word, "word").expect("cool");
    assert_eq!(
        vec!["word".to_string()],
        pairs.clone().next().unwrap().to_strings()
    );

    pairs = ShellParser::parse(Rule::word, "word\\\\").expect("cool");
    assert_eq!(
        vec!["word\\".to_string()],
        pairs.clone().next().unwrap().to_strings()
    );

    std::env::set_var("COOLNAME", "cewl");
    pairs = ShellParser::parse(Rule::variable, "$COOLNAME").expect("cool");
    assert_eq!(
        vec!["cewl".to_string()],
        pairs.clone().next().unwrap().to_strings()
    );

    pairs = ShellParser::parse(Rule::argument, "aaa$COOLNAME").expect("cool");
    assert_eq!(
        vec!["aaacewl".to_string()],
        pairs.clone().next().unwrap().to_strings()
    );

    pairs = ShellParser::parse(Rule::argument_list, r#""wow" aaa $COOLNAME"#).expect("cool");
    assert_eq!(
        vec!["wow".to_string(), "aaa".to_string(), "cewl".to_string()],
        pairs.clone().next().unwrap().to_strings()
    );

    pairs =
        ShellParser::parse(Rule::argument_list, r#""wow $COOLNAME" aaa $COOLNAME"#).expect("cool");
    assert_eq!(
        vec![
            "wow cewl".to_string(),
            "aaa".to_string(),
            "cewl".to_string()
        ],
        pairs.clone().next().unwrap().to_strings()
    );
}

trait ToStringVec {
    fn to_strings(&mut self) -> Vec<String>;
    fn process_children(&mut self) -> Vec<String>;
    fn get_input(&mut self) -> Option<Stdio>;
    fn get_output(&mut self) -> Option<Stdio>;
}

impl ToStringVec for Pair<'_, Rule> {
    fn process_children(&mut self) -> Vec<String> {
        self.clone()
            .into_inner()
            .flat_map(|x| x.clone().to_strings())
            .collect()
    }
    fn to_strings(&mut self) -> Vec<String> {
        match self.as_rule() {
            Rule::argument_list => self.process_children(),

            Rule::argument
            | Rule::variable
            | Rule::word
            | Rule::double_quoted_word
            | Rule::double_quoted_inner
            | Rule::single_quoted_inner
            | Rule::single_quoted_word
            | Rule::escaped_char => vec![self.process_children().join("")],

            Rule::regular_char | Rule::escaped_tail => vec![self.as_str().to_string()],

            Rule::variable_name => vec![var_or_empty(self)],

            Rule::double_quoted_trivia | Rule::space | Rule::single_quoted_trivia => {
                vec![self.as_str().to_string()]
            }
            _ => vec![],
        }
    }

    fn get_input(&mut self) -> Option<Stdio> {
        match self.as_rule() {
            Rule::argument_list | Rule::redirect | Rule::redirect_input => {
            self
                .clone()
                .into_inner()
                .filter_map(|x| x.clone().get_input())
                .nth(0)
            },
            Rule::filename => Some(Stdio::from(
                File::open(self.as_str()).expect("FileNotFound"),
            )),
            _ => None,
        }
    }

    fn get_output(&mut self) -> Option<Stdio> {
        match self.as_rule() {
            Rule::argument_list | Rule::redirect | Rule::redirect_output => {
            self
                .clone()
                .into_inner()
                .filter_map(|x| x.clone().get_output())
                .nth(0)
            },
            Rule::filename => Some(Stdio::from(
                File::create(self.as_str()).expect("FileNotFound"),
            )),
            _ => None,
        }
    }
}

fn var_or_empty(pair: &mut Pair<'_, Rule>) -> String {
    var(pair.as_str())
        .or_else::<VarError, _>(|_| Ok("".to_string()))
        .unwrap()
}

fn parse(line: &String) -> (Vec<String>, Option<Stdio>, Option<Stdio>) {
    let pairs = ShellParser::parse(Rule::argument_list, line).expect("shiiit");
    let string_vec = pairs.clone().flat_map(|p| p.clone().to_strings()).collect();
    let input = pairs
        .clone()
        .map(|p| p.clone().get_input())
        .nth(0)
        .unwrap_or_default();

    let output = pairs
        .clone()
        .map(|p| p.clone().get_output())
        .nth(0)
        .unwrap_or_default();
    (string_vec, input, output)
}

fn exit_cmd() {
    exit(0);
}

fn parse_command(s: &str) -> ShellCommand {
    match s {
        "exit" => ShellCommand::InternalCommand(InternalCommand::Exit),
        _ => ShellCommand::ProgramName(String::from(s)),
    }
}

#[test]
fn test_parse() {
    assert_eq!(parse(&String::from("1 2 3")).0, vec!["1", "2", "3"]);
}
