use std::io;
use std::io::prelude::*;
use std::process::*;

extern crate regex;
use regex::Regex;


#[macro_use]
extern crate pest_derive;

extern crate pest;
use pest::Parser;

#[derive(Parser)]
#[grammar="grammar.pest"]
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
    let parsed_args = parse(line);
    let command = match parsed_args.first() {
        Some(x) => x,
        None => return,
    };
    let parsed_command = parse_command(command);

    match parsed_command {
        ShellCommand::ProgramName(path) => execute_program(path, parsed_args),
        ShellCommand::InternalCommand(ic) => execute_internal_program(ic),
    }
}

fn execute_program(path: String, args: Vec<&str>) {
    match std::process::Command::new(path)
        .args(args.iter().skip(1))
        .stdout(Stdio::inherit())
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

fn parse(line: &String) -> Vec<&str> {
    let pairs = ShellParser::parse(Rule::argument_list, line).expect("shiiit");
    pairs.flat_map(|p| p.into_inner())
         .filter( |x|  match x.as_rule() {  Rule::argument => true, _ => false })
         .flat_map(|x| x.into_inner())
         .map(|x| argument_to_string(&x))
         .collect()
}

fn argument_to_string<'a>(pair: &pest::iterators::Pair<'a,Rule>) -> &'a str{
    match pair.as_rule() {
        Rule::double_quoted_word => pair.clone().into_inner().nth(1).unwrap().as_str(),
        Rule::single_quoted_word => pair.clone().into_inner().nth(1).unwrap().as_str(),
        _ => pair.as_str()
    }
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
    assert_eq!(parse(&String::from("1 2 3")), vec!["1", "2", "3"]);
}
