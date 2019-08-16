use std::env::VarError;
use std::io;
use std::io::prelude::*;
use std::process::*;

extern crate regex;

#[macro_use]
extern crate pest_derive;

extern crate pest;
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

fn execute_program(path: String, args: Vec<String>) {
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

/* fn find_all_tokens_of_type<'a, 'b>(
    root: &'b pest::iterators::Pair<'a, Rule>,
    rule: Rule,
) -> Vec<pest::iterators::Pair<'a, Rule>> {
    if root.as_rule() == rule {
        return vec![root.clone()];
    }
    root.clone()
        .into_inner()
        .flat_map(|x| find_all_tokens_of_type(&x, rule))
        .collect()
} */

#[test]
fn test_to_strings() {
    let mut pairs = ShellParser::parse(Rule::word, "word").expect("cool");
    assert_eq!(
        vec!["word".to_string()],
        to_strings(pairs.clone().next().unwrap())
    );

    pairs = ShellParser::parse(Rule::word, "word\\\\").expect("cool");
    assert_eq!(
        vec!["word\\".to_string()],
        to_strings(pairs.clone().next().unwrap())
    );

    std::env::set_var("COOLNAME", "cewl");
    pairs = ShellParser::parse(Rule::variable, "$COOLNAME").expect("cool");
    assert_eq!(
        vec!["cewl".to_string()],
        to_strings(pairs.clone().next().unwrap())
    );

    pairs = ShellParser::parse(Rule::argument, "aaa$COOLNAME").expect("cool");
    assert_eq!(
        vec!["aaacewl".to_string()],
        to_strings(pairs.clone().next().unwrap())
    );

    pairs = ShellParser::parse(Rule::argument_list, r#""wow" aaa $COOLNAME"#).expect("cool");
    assert_eq!(
        vec!["wow".to_string(), "aaa".to_string(), "cewl".to_string()],
        to_strings(pairs.clone().next().unwrap())
    );

    pairs =
        ShellParser::parse(Rule::argument_list, r#""wow $COOLNAME" aaa $COOLNAME"#).expect("cool");
    assert_eq!(
        vec![
            "wow cewl".to_string(),
            "aaa".to_string(),
            "cewl".to_string()
        ],
        to_strings(pairs.clone().next().unwrap())
    );
}

fn to_strings(pair: pest::iterators::Pair<'_, Rule>) -> Vec<String> {
    match pair.as_rule() {
        Rule::argument_list => pair
            .clone()
            .into_inner()
            .flat_map(|x| to_strings(x))
            .collect(),
        Rule::argument
        | Rule::double_quoted_word
        | Rule::double_quoted_inner
        | Rule::single_quoted_inner
        | Rule::single_quoted_word
        | Rule::escaped_char => {
            let inner_vec: Vec<String> = pair
                .clone()
                .into_inner()
                .flat_map(|x| to_strings(x))
                .collect();
            vec![inner_vec.join("")]
        }
        Rule::word => {
            let mut to_return = vec![];
            for x in pair.into_inner() {
                to_return.append(&mut to_strings(x));
            }
            vec![to_return.join("")]
        }
        Rule::regular_char | Rule::escaped_tail => vec![pair.as_str().to_string()],
        Rule::variable_name => vec![std::env::var(pair.as_str())
            .or_else::<VarError, _>(|_| Ok("".to_string()))
            .unwrap()],
        Rule::variable => to_strings(pair.clone().into_inner().nth(1).unwrap()),
        Rule::double_quoted_trivia | Rule::space | Rule::single_quoted_trivia => {
            vec![pair.as_str().to_string()]
        }
        _ => vec![],
    }
}

fn parse(line: &String) -> Vec<String> {
    let pairs = ShellParser::parse(Rule::argument_list, line).expect("shiiit");
    pairs.flat_map(|p| to_strings(p)).collect()
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
