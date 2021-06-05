// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

pub static XPM: &[&str] = &[
    "64 64 15 1",
    " 	c None",
    "1	c #8D8D8D",
    "2	c #00DB00",
    "3	c #E9E9E9",
    "4	c #2D2D2D",
    "5	c #7C7C7C",
    "6	c #C9C9C9",
    "7	c #616161",
    "8	c #111111",
    "9	c #FEFEFE",
    "A	c #3F3F3F",
    "B	c #D7D7D7",
    "C	c #B1B1B1",
    "D	c #F3F3F3",
    "E	c #9F9F9F",
    "                                                                ",
    "                                                                ",
    "                                                                ",
    "            6666666666666666666666666666666666666666            ",
    "           688888888888888888888888888888888888888886           ",
    "           682222222222222222222222222222222222222286           ",
    "           682222222222222222222222222222222222222286           ",
    "           682222222222222222222222222222222222222286           ",
    "           682222222222222222222222222222222222222286           ",
    "           682222222222222222222222222222222222222286           ",
    "           682222222222222222222222222222222222222286           ",
    "           B4444444444444444444884444444444444444444B           ",
    "            33333333333333333D6776D33333333333333333            ",
    "                              6446                              ",
    "                             B4558B                             ",
    "                             585558                             ",
    "                           D18555581D                           ",
    "                           3855555583                           ",
    "                          358555555883                          ",
    "                          185555555588                          ",
    "                         BA5555555555AB                         ",
    "                         75555555555554                         ",
    "                        6455555555555546                        ",
    "                       345555555555555543                       ",
    "                       C85555555555555588                       ",
    "                      C4555555555555555548                      ",
    "                      75555555555555555558                      ",
    "                     1855555555555555555588                     ",
    "                     7555555555555555555558                     ",
    "                    585555555555555555555588                    ",
    "                   68555555555555555555555586                   ",
    "                  B78555555555555555555555558B                  ",
    "                  E855555555555555555555555558                  ",
    "                 CA55555555555555555555555555A8                 ",
    "                 785555555555555555555555555558                 ",
    "                64555555555555555555555555555546                ",
    "               3A55555555555555555555555555555583               ",
    "              D4555555555555555555555555555555558D              ",
    "              C4555555555555555555555555555555558C              ",
    "              455555555555555555555555555555555558              ",
    "             55555555555555555555555555555555555588             ",
    "            DA555555555555555555555555555555555555AD            ",
    "           385555555555555555555555555555555555555583           ",
    "           BA84444444444855555555555555844444444448AB           ",
    "           D666BBBBBBBBC4555555555555554CBBBBBBBB666D           ",
    "                       B4555555555555554B                       ",
    "                       B4555555555555554B                       ",
    "                       B4555555555555554B                       ",
    "                       C8555555555555558C                       ",
    "                      DE8555555555555558ED                      ",
    "                       E8555555555555558E                       ",
    "                      DE8555555555555558ED                      ",
    "                      DE8555555555555558ED                      ",
    "                       E8555555555555558E                       ",
    "                       E8555555555555558E                       ",
    "                       E8555555555555558E                       ",
    "                       E8555555555555558E                       ",
    "                       E8555555555555558E                       ",
    "                       E8555555555555558E                       ",
    "                       CAAAAAAAAAAAAAAAAC                       ",
    "                        DDDDDDDDDDDDDDDD                        ",
    "                                                                ",
    "                                                                ",
    "                                                                ",
];

use pw_gtk_ext::{gdk_pixbuf, gtk};

#[allow(dead_code)]
pub fn pixbuf() -> gdk_pixbuf::Pixbuf {
    gdk_pixbuf::Pixbuf::from_xpm_data(XPM)
}

#[allow(dead_code)]
pub fn sized_pixbuf(size: i32) -> Option<gdk_pixbuf::Pixbuf> {
    pixbuf().scale_simple(size, size, gdk_pixbuf::InterpType::Bilinear)
}

#[allow(dead_code)]
pub fn sized_pixbuf_or(size: i32) -> gdk_pixbuf::Pixbuf {
    if let Some(pixbuf) = sized_pixbuf(size) {
        pixbuf
    } else {
        pixbuf()
    }
}

#[allow(dead_code)]
pub fn image() -> gtk::Image {
    gtk::Image::from_pixbuf(Some(&pixbuf()))
}

#[allow(dead_code)]
pub fn sized_image(size: i32) -> Option<gtk::Image> {
    if let Some(pixbuf) = pixbuf().scale_simple(size, size, gdk_pixbuf::InterpType::Bilinear) {
        Some(gtk::Image::from_pixbuf(Some(&pixbuf)))
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn sized_image_or(size: i32) -> gtk::Image {
    if let Some(image) = sized_image(size) {
        image
    } else {
        image()
    }
}
