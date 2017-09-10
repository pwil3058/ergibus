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

#[macro_use]
extern crate serde_derive;

extern crate chrono;
extern crate crypto_hash;
extern crate globset;
extern crate hex;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
extern crate snap;
extern crate walkdir;

mod archive;
mod content;
mod pathux;
mod report;
pub mod snapshot;
