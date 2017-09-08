extern crate argparse;

extern crate ergibus;
//mod snapshot;

use std::io::{stdout, stderr};
use std::path::PathBuf;
use std::str::FromStr;

use argparse::{ArgumentParser, StoreTrue, Store, List};

use ergibus::snapshot;

#[allow(non_camel_case_types)]
#[derive(Debug)]
enum Command {
    backup,
    delete,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(src: &str) -> Result<Command, ()> {
        return match src {
            "backup" | "bu" => Ok(Command::backup),
            "delete" | "del" => Ok(Command::delete),
            _ => Err(()),
        };
    }
}

fn backup_command(args: Vec<String>) {
    let mut archive = "".to_string();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Takes a back up snapshot");
        ap.refer(&mut archive).required()
            .add_option(&["--archive"], Store,
                r#"name of archive specifying what to back up"#);
        match ap.parse(args, &mut stdout(), &mut stderr()) {
            Ok(()) =>  {}
            Err(x) => {
                std::process::exit(x);
            }
        }
    }
    match snapshot::generate_snapshot(&archive) {
        Ok(()) => {}
        Err(err) => {
            println!("{:?}", err);
            std::process::exit(1);
        }
    }
}

fn delete_command(args: Vec<String>) {
    let mut file = "".to_string();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Deletes a snapshot file");
        ap.refer(&mut file)
            .add_option(&["--file"], Store,
                "Path of snapshot file to delete");
        match ap.parse(args, &mut stdout(), &mut stderr()) {
            Ok(()) =>  {}
            Err(x) => {
                std::process::exit(x);
            }
        }
    }
    let path = PathBuf::from(file);
    match snapshot::delete_snapshot_file(&path) {
        Ok(()) => {}
        Err(err) => {
            println!("{:?}", err);
            std::process::exit(1);
        }
    }
}

fn main() {
    let mut subcommand = Command::backup;
    let mut args = vec!();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Manage file back ups");
        ap.refer(&mut subcommand).required()
            .add_argument("command", Store,
                "Command to run (either \"backup\" or \"delete\")");
        ap.refer(&mut args)
            .add_argument("arguments", List,
                "Arguments for command");
        ap.stop_on_first_argument(true);
        ap.parse_args_or_exit();
    }

    args.insert(0, format!("ergibus {:?}", subcommand));
    match subcommand {
        Command::backup => backup_command(args),
        Command::delete => delete_command(args),
    }
}
