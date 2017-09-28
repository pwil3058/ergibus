// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod combo_box_text;
pub mod list_store;

pub mod recollect {
    use std::collections::HashMap;
    use std::fs;
    use std::io::{self, Write, Seek};
    use std::path;

    use fs2::FileExt;
    use serde_json;

    type RDB = HashMap<String, String>;

    pub struct Recollections {
        file_path: path::PathBuf
    }

    impl Recollections {
        pub fn new(file_path: &path::Path) -> Recollections {
            if !file_path.exists() {
                if let Some(dir_path) = file_path.parent() {
                    if !dir_path.exists() {
                        fs::create_dir_all(&dir_path).unwrap_or_else(
                            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
                        );
                    }
                }
                let file = fs::File::create(file_path).unwrap_or_else(
                    |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
                );
                serde_json::to_writer(&file, &RDB::new()).unwrap_or_else(
                    |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
                );
            }
            Recollections{file_path: file_path.to_path_buf()}
        }

        pub fn recall(&self, name: &str) -> Option<String> {
            let file = fs::File::open(&self.file_path).unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            file.lock_shared().unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            let hash_map: RDB = serde_json::from_reader(&file).unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            file.unlock().unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            match hash_map.get(name) {
                Some(ref s) => Some(s.to_string()),
                None => None
            }
        }

        pub fn recall_or_else(&self, name: &str, default: &str) -> String {
            match self.recall(name) {
                Some(string) => string,
                None => default.to_string()
            }
        }

        pub fn remember(&self, name: &str, value: &str) {
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.file_path).unwrap_or_else(
                    |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
                );
            file.lock_exclusive().unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            let mut hash_map: RDB = serde_json::from_reader(&file).unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            hash_map.insert(name.to_string(), value.to_string());
            file.seek(io::SeekFrom::Start(0)).unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            file.set_len(0).unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            serde_json::to_writer(&file, &hash_map).unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            file.unlock().unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path;

    #[test]
    fn recollection_test() {
        let recollection_file = path::Path::new(".recollection_test");
        let recollections = recollect::Recollections::new(&recollection_file);
        assert_eq!(recollections.recall("anything"), None);
        assert_eq!(recollections.recall_or_else("anything", "but"), "but");
        recollections.remember("anything", "whatever");
        assert_eq!(recollections.recall("anything"), Some("whatever".to_string()));
        assert_eq!(recollections.recall_or_else("anything", "but"), "whatever");
        fs::remove_file(recollection_file);
    }
}
