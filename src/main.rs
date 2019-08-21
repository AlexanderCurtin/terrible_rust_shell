use std::env::var;
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
    let command_lines = ShellParser::parse(Rule::command_line, line).expect("gotta give me a command line");

    for command_line in command_lines {
        match process_command_line(command_line) {
            Err(x) => println!("{}", x),
            _ => ()
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
        pairs.next().unwrap().get_args()
    );

    pairs = ShellParser::parse(Rule::word, "word\\\\").expect("cool");
    assert_eq!(
        vec!["word\\".to_string()],
        pairs.next().unwrap().get_args()
    );

    std::env::set_var("COOLNAME", "cewl");
    pairs = ShellParser::parse(Rule::variable, "$COOLNAME").expect("cool");
    assert_eq!(
        vec!["cewl".to_string()],
        pairs.next().unwrap().get_args()
    );

    pairs = ShellParser::parse(Rule::argument, "aaa$COOLNAME").expect("cool");
    assert_eq!(
        vec!["aaacewl".to_string()],
        pairs.next().unwrap().get_args()
    );

    pairs = ShellParser::parse(Rule::argument_list, r#""wow" aaa $COOLNAME"#).expect("cool");
    assert_eq!(
        vec!["wow".to_string(), "aaa".to_string(), "cewl".to_string()],
        pairs.next().unwrap().get_args()
    );

    pairs =
        ShellParser::parse(Rule::argument_list, r#""wow $COOLNAME" aaa $COOLNAME"#).expect("cool");
    assert_eq!(
        vec![
            "wow cewl".to_string(),
            "aaa".to_string(),
            "cewl".to_string()
        ],
        pairs.next().unwrap().get_args()
    );
}

trait ParserHelpers {
    fn get_args(self) -> Vec<String>;
    fn process_children(self) -> Vec<String>;
    fn get_input(self) -> Option<Stdio>;
    fn get_output(self) -> Option<Stdio>;
}

impl ParserHelpers for Pair<'_, Rule> {
    fn process_children(self) -> Vec<String> {
        self.into_inner()
            .flat_map(|x| x.get_args())
            .collect()
    }
    fn get_args(self) -> Vec<String> {
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

    fn get_input(self) -> Option<Stdio> {
        match self.as_rule() {
            Rule::argument_list | Rule::redirect | Rule::redirect_input => self
                .into_inner()
                .find_map(|x| x.get_input()),
            Rule::filename => Some(Stdio::from(
                File::open(self.as_str()).expect("FileNotFound"),
            )),
            _ => None,
        }
    }

    fn get_output(self) -> Option<Stdio> {
        match self.as_rule() {
            Rule::argument_list | Rule::redirect | Rule::redirect_output => self
                .into_inner()
                .find_map(|x| x.get_output()),
            Rule::filename => Some(Stdio::from(
                File::create(self.as_str()).expect("FileNotFound"),
            )),
            _ => None,
        }
    }
}

fn var_or_empty(pair: Pair<'_, Rule>) -> String {
    var(pair.as_str()).unwrap_or_default()
}

fn process_command_line(pair: Pair<'_, Rule>) -> Result<(), String> {
    assert_eq!(pair.as_rule(), Rule::command_line);
    let mut pairs = pair.into_inner();
    let mut commands: Vec<Child> = vec![];
    let mut last_reader: Option<Stdio> = None;
    while let Some(cur_pair) = pairs.next() {
        assert_eq!(cur_pair.as_rule(), Rule::argument_list);

        let (args, parsed_input, parsed_output) =
            parse(&cur_pair.as_str().to_string());

        let parsed_command = parse_command(args.first().unwrap().as_str());

        let _x = match parsed_command {
            ShellCommand::InternalCommand(x) => return Ok(execute_internal_program(x)),
            _ => (),
        };

        let mut passed_input = None;
        std::mem::swap(&mut last_reader, &mut passed_input);

        let current_input = select_current_input(parsed_input, passed_input);

        let mut current_output = None;

        if parsed_output.is_none() {
            let (stdout, reader) = match pairs.peek() {
                None => (None, None),
                Some(_) => {
                    pairs.next();
                    let (pipe_reader, pipe_writer) = os_pipe::pipe().unwrap();
                    (
                        Some(Stdio::from(pipe_writer)),
                        Some(Stdio::from(pipe_reader)),
                    )
                }
            };
            current_output = stdout.or_else(|| Some(Stdio::inherit()));
            last_reader = reader;
        }

        let current_cmd = Command::new(args.first().unwrap())
            .args(args.iter().skip(1))
            .stdin(current_input.unwrap())
            .stdout(current_output.unwrap())
            .spawn();

        let cmd = current_cmd.expect("I need this to work");
        commands.push(cmd);
    }
    commands.iter_mut().try_for_each(|x| {
        match x.wait() {
            Ok(_) => Ok(()),
            Err(a) => Err(a)
        }
    }).unwrap_or_else(|_| eprintln!("something went wrong"));
    io::stdout().flush().unwrap();

    Ok(())
}


fn select_current_input(parsed_input: Option<Stdio>, passed_input: Option<Stdio>) -> Option<Stdio>{
    return parsed_input.or_else(|| passed_input).or_else(|| Some(Stdio::inherit()));
}

fn parse(line: &String) -> (Vec<String>, Option<Stdio>, Option<Stdio>) {
    let pairs = ShellParser::parse(Rule::argument_list, line).expect("shiiit");
    let string_vec = pairs.clone().flat_map(|p| p.get_args()).collect();

    let input = pairs.clone().find_map(|p| p.get_input());

    let output = pairs.clone().find_map(|p| p.get_output());
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
