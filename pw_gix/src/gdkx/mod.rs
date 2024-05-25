// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::num::ParseIntError;

use gdk;

pub fn format_geometry(event: &gdk::EventConfigure) -> String {
    let (x, y) = event.get_position();
    let (w, h) = event.get_size();
    format!("{}x{}+{}+{}", w, h, x, y)
}

pub fn format_geometry_size(event: &gdk::EventConfigure) -> String {
    let (w, h) = event.get_size();
    format!("{}x{}", w, h)
}

pub fn format_geometry_position(event: &gdk::EventConfigure) -> String {
    let (x, y) = event.get_position();
    format!("{}+{}", x, y)
}

pub fn parse_geometry(text: &str) -> Result<(i32, i32, i32, i32), ParseIntError> {
    let v: Vec<&str> = text.splitn(2, '+').collect();
    let (width, height) = parse_geometry_size(v[0])?;
    let (x, y) = parse_geometry_position(v[1])?;
    Ok((width, height, x, y))
}

pub fn parse_geometry_size(text: &str) -> Result<(i32, i32), ParseIntError> {
    let v: Vec<&str> = text.splitn(2, 'x').collect();
    let width = v[0].parse::<i32>()?;
    let height = v[1].parse::<i32>()?;
    Ok((width, height))
}

pub fn parse_geometry_position(text: &str) -> Result<(i32, i32), ParseIntError> {
    let v: Vec<&str> = text.splitn(2, '+').collect();
    let x = v[0].parse::<i32>()?;
    let y = v[1].parse::<i32>()?;
    Ok((x, y))
}
