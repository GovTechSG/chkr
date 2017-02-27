extern crate docopt;
extern crate md5;
extern crate csv;
extern crate rustc_serialize;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

mod checksum;
use checksum::{ChecksumResult, Outcome};

use std::path::PathBuf;
use std::fs::canonicalize;

use docopt::Docopt;

const USAGE: &'static str = r##"
          oooo        oooo
          `888        `888
 .ooooo.   888 .oo.    888  oooo  oooo d8b
d88' `"Y8  888P"Y88b   888 .8P'   `888""8P
888        888   888   888888.     888
888   .o8  888   888   888 `88b.   888
`Y8bod8P' o888o o888o o888o o888o d888b

Usage:
  chkr file <file-path> <expected-checksum>
  chkr manifest <checksum-path>
  chkr (-h | --help)

chkr will return 0 for matches, 0x01 for mismatch, and 0x10 for other errors.

Options:
  -h --help     Show this screen.
"##;

#[derive(Debug, RustcDecodable)]
struct Args {
    cmd_file: bool,
    cmd_manifest: bool,
    arg_expected_checksum: String,
    arg_file_path: String,
    arg_checksum_path: String,
}

enum ReturnCode {
    Ok = 0x00,
    Mismatch = 0x01,
    Error = 0x02,
}

fn main() {
    let exit_code;
    {
        let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.decode())
            .unwrap_or_else(|e| e.exit());

        let command = get_command(&args);
        exit_code = match command {
            Some(cmd) => cmd(&args),
            None => {
                println!("Unknown command");
                ReturnCode::Error as u8
            }
        };
    }
    std::process::exit(exit_code as i32);
}

fn get_command(args: &Args) -> Option<fn(&Args) -> u8> {
    match args {
        &Args { cmd_file: true, .. } => Some(file),
        &Args { cmd_manifest: true, .. } => Some(manifest),
        _ => None,
    }
}

fn file(args: &Args) -> u8 {
    let file_path = canonicalize(&PathBuf::from(&args.arg_file_path));
    if let Err(e) = file_path {
        println!("Error verifying checksum: {:?}", e);
        return ReturnCode::Error as u8;
    }

    let file_path = file_path.unwrap();
    match checksum::verify_checksum(&file_path, &args.arg_expected_checksum) {
        Err(e) => {
            println!("Error verifying checksum for  {:?}: {}", file_path, e);
            ReturnCode::Error as u8
        }
        Ok(Outcome::Match) => {
            println!("{:?} checksum matched", file_path);
            ReturnCode::Ok as u8
        }
        Ok(outcome @ Outcome::Mismatch { .. }) => {
            println!("{:?} checksum mismatch: {:?}", file_path, outcome);
            ReturnCode::Mismatch as u8
        }
    }
}

fn manifest(args: &Args) -> u8 {
    let result = checksum::verify_checksums_file(&args.arg_checksum_path);
    if let Err(e) = result {
        println!("Error verifying checksum: {}", e);
        return ReturnCode::Error as u8;
    }
    let result = result.unwrap();
    let total_length = result.len as f32;
    let mut i = 0u32;

    result.fold(ReturnCode::Ok as u8, |return_code, item| {
        i = i + 1;
        let percent_done = (i as f32) / (total_length) * 100.;
        let progress_text = format!("({}/{} {:.2}%)", i, total_length, percent_done);

        return_code |
        (match item {
            Ok(ChecksumResult { file, result: Err(e) }) => {
                println!("{} {}: Error: {}", progress_text, file, e);
                ReturnCode::Error
            }
            Ok(ChecksumResult { file, result: outcome @ Ok(Outcome::Mismatch { .. }) }) => {
                println!("{} {}: Error: {:?}", progress_text, file, outcome);
                ReturnCode::Mismatch
            }
            Ok(ChecksumResult { file, result: Ok(outcome) }) => {
                println!("{} {}: {:?}", progress_text, file, outcome);
                ReturnCode::Ok // nop, assumes Ok is 0u8
            }
            Err(error) => {
                println!("{} Error: {}", progress_text, error);
                ReturnCode::Error
            }
        } as u8)
    })
}

#[cfg(test)]
mod tests {
    use super::{file, manifest, Args};

    #[test]
    fn file_returns_zero_for_match() {
        let args = Args {
            cmd_file: true,
            cmd_manifest: false,
            arg_expected_checksum: "4d93d51945b88325c213640ef59fc50b".to_string(),
            arg_file_path: "tests/fixtures/foo.txt".to_string(),
            arg_checksum_path: "".to_string(),
        };

        assert_eq!(file(&args), 0);
    }

    #[test]
    fn file_returns_one_for_mismatch() {
        let args = Args {
            cmd_file: true,
            cmd_manifest: false,
            arg_expected_checksum: "4d93d51945b88325c213640ef59fc50a".to_string(),
            arg_file_path: "tests/fixtures/bar.txt".to_string(),
            arg_checksum_path: "".to_string(),
        };

        assert_eq!(file(&args), 1);
    }

    #[test]
    fn file_returns_two_for_errors() {
        let args = Args {
            cmd_file: true,
            cmd_manifest: false,
            arg_expected_checksum: "ce5188defed222ca612b41580e0d5fe6".to_string(),
            arg_file_path: "tests/fixtures/does-not-exist.csv".to_string(),
            arg_checksum_path: "".to_string(),
        };

        assert_eq!(file(&args), 2);
    }

    #[test]
    fn manifest_returns_three_for_fixture() {
        let args = Args {
            cmd_file: false,
            cmd_manifest: true,
            arg_expected_checksum: "".to_string(),
            arg_file_path: "".to_string(),
            arg_checksum_path: "tests/fixtures/checksum.txt".to_string(),
        };

        assert_eq!(manifest(&args), 3);
    }
}
