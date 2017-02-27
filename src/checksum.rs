use std::fs::{canonicalize, File};
use std::io::Read;
use std::path::PathBuf;
use std::vec;

use md5;
use csv;

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub enum Outcome {
    Match,
    Mismatch { expected: String, actual: String },
}

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub struct ChecksumRecord {
    pub file: String,
    pub checksum: String,
}

#[derive(Clone, Debug, RustcDecodable, PartialEq)]
pub struct ChecksumResult {
    pub file: String,
    pub result: Result<Outcome, String>,
}

pub struct ChecksumResultsIter {
    iterator: vec::IntoIter<Result<ChecksumRecord, String>>,
    working_directory: PathBuf,
    pub len: usize,
}

impl Iterator for ChecksumResultsIter {
    type Item = Result<ChecksumResult, String>;

    fn next(&mut self) -> Option<Result<ChecksumResult, String>> {
        match self.iterator.next() {
            None => None,
            Some(Ok(record)) => {
                let ChecksumRecord { file: ref relative_path, checksum: ref expected_checksum } = record;
                let file_path = self.working_directory.join(relative_path);
                Some(Ok(ChecksumResult {
                    file: relative_path.clone(),
                    result: verify_checksum(&file_path, expected_checksum),
                }))
            }
            Some(Err(e)) => Some(Err(e)),
        }
    }
}

pub fn read_checksums(path: &str) -> Result<Vec<Result<ChecksumRecord, String>>, String> {
    let checksum_reader = csv::Reader::from_file(path).map_err(|e| format!("{:?}", e))?;

    let mut checksum_reader = checksum_reader.delimiter(b' ').has_headers(false);

    let checksums = checksum_reader.records()
        .map(|row| {
            // The files are probably created by the `md5sum` utility
            // Two spaces are used to delimit
            match row {
                Ok(row_unwrapped) => {
                    Ok(ChecksumRecord {
                        file: row_unwrapped[2].clone(),
                        checksum: row_unwrapped[0].clone(),
                    })
                }
                Err(e) => Err(format!("{:?}", e)),
            }
        })
        .filter(|row| match *row {
            Err(_) => true,
            Ok(ref row) => !row.file.is_empty() && !row.checksum.is_empty(),
        })
        .collect();
    Ok(checksums)
}

pub fn verify_checksum(path: &PathBuf, expected_digest: &str) -> Result<Outcome, String> {
    let file_buffer = read(path).map_err(|e| format!("{}", e))?;

    let actual_digest = md5::compute(file_buffer);
    let actual_digest = format!("{:x}", actual_digest);
    if actual_digest == expected_digest {
        Ok(Outcome::Match)
    } else {
        Ok(Outcome::Mismatch {
            expected: expected_digest.to_string(),
            actual: actual_digest.to_string(),
        })
    }
}

/// Verify checksums of files according to a manifest file.
pub fn verify_checksums_file(file: &str) -> Result<ChecksumResultsIter, String> {
    let checksums_path = canonicalize(&PathBuf::from(file)).map_err(|e| format!("{}", e))?;
    let working_directory = checksums_path.parent()
        .ok_or("Unable to compute working directory".to_string())?;
    let checksums_path = checksums_path.as_path()
        .to_str()
        .ok_or("Unable to convert paths".to_string())?;
    let checksums = read_checksums(checksums_path)?;

    Ok(ChecksumResultsIter {
        working_directory: working_directory.to_path_buf(),
        len: checksums.len(),
        iterator: checksums.into_iter(),
    })
}

fn read(path: &PathBuf) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::<u8>::new();
    let mut f = File::open(path).map_err(|e| format!("{:?}", e))?;
    f.read_to_end(&mut buffer).map_err(|e| format!("{:?}", e))?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::{read_checksums, Outcome, ChecksumRecord, verify_checksum, verify_checksums_file, ChecksumResult};
    use std::path::PathBuf;
    use std::vec::Vec;
    use std::collections::HashMap;

    #[test]
    fn checksums_are_read_correctly() {
        let expected_checksums = vec![Ok(ChecksumRecord {
                                          file: "foo.txt".to_string(),
                                          checksum: "4d93d51945b88325c213640ef59fc50b".to_string(),
                                      }),
                                      Ok(ChecksumRecord {
                                          file: "bar.txt".to_string(),
                                          checksum: "4d93d51945b88325c213640ef59fc50a".to_string(),
                                      }),
                                      Ok(ChecksumRecord {
                                          file: "file-does-not-exist".to_string(),
                                          checksum: "ce5188defed222ca612b41580e0d5fe7".to_string(),
                                      })];
        let actual_checksums = read_checksums("tests/fixtures/checksum.txt").unwrap();

        assert_eq!(expected_checksums, actual_checksums);
    }

    #[test]
    fn checksum_is_verified_correctly() {
        let actual_result = verify_checksum(&PathBuf::from("tests/fixtures/foo.txt"),
                                            &"4d93d51945b88325c213640ef59fc50b");

        assert_matches!(actual_result, Ok(Outcome::Match));
    }

    #[test]
    fn incorrect_checksum_is_verified_correctly() {
        let actual_result = verify_checksum(&PathBuf::from("tests/fixtures/foo.txt"),
                                            &"ce5188defed222ca612b41580e0d5fe6");
        assert_matches!(actual_result, Ok(Outcome::Mismatch { .. }));
    }

    #[test]
    fn missing_file_is_reported() {
        let actual_result = verify_checksum(&PathBuf::from("tests/fixtures/non-existent-file"),
                                            &"ce5188defed222ca612b41580e0d5fe6");
        assert_matches!(actual_result, Err(_));
    }

    #[test]
    fn checksums_manifest_is_verified_correctly() {
        let actual_result: Vec<Result<ChecksumResult, String>> = verify_checksums_file("tests/fixtures/checksum.txt")
            .unwrap()
            .collect();

        let errors: Vec<String> = actual_result.iter()
            .filter(|r| r.is_err())
            .map(|r| r.as_ref().unwrap_err().clone())
            .collect();
        assert_eq!(errors, Vec::<String>::new());

        let results: HashMap<String, Result<Outcome, String>> = actual_result.iter()
            .filter(|r| r.is_ok())
            .map(|r| {
                let r = r.as_ref().unwrap();
                (r.file.clone(), r.result.clone())
            })
            .collect();
        let mut actual_keys: Vec<String> = results.keys().map(|key| key.clone()).collect();
        actual_keys.sort();
        let mut expected_keys = vec!["bar.txt", "file-does-not-exist", "foo.txt"];
        expected_keys.sort();
        assert_eq!(actual_keys, expected_keys);

        assert_matches!(results.get("foo.txt").unwrap(), &Ok(Outcome::Match));
        assert_matches!(results.get("bar.txt").unwrap(), &Ok(Outcome::Mismatch{ .. }));
        assert_matches!(results.get("file-does-not-exist").unwrap(), &Err(_));
    }
}
