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

extern crate chrono;
//#[macro_use]
extern crate clap;
extern crate crypto_hash;
extern crate fs2;
extern crate globset;
extern crate hex;
extern crate hostname;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate pw_gix;
extern crate pw_pathux;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;
extern crate snap;
extern crate tempdir;
extern crate users;
extern crate walkdir;

extern crate gdk;
extern crate gtk;

pub mod archive;
pub mod attributes;
pub mod config;
pub mod content;
mod eerror;
//pub mod pathux;
mod report;
pub mod snapshot;

//pub mod gdkx;
//pub mod gtkx;

pub mod cli;
pub mod gui;
