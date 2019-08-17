#![feature(test)]

extern crate test;

use gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gio::prelude::*;
use gtk::prelude::*;
use gtk::Image;
use gtk::{Application, ApplicationWindow, Button, EventBox, ScrolledWindow};
use std::io::BufReader;

use crate::mandelbrot::{mandelbrot, Window};

mod mandelbrot;

fn bytes_to_image_buffer(bytes: &[u8], pixbuf: &Pixbuf) {
    let (wid, hei) = (pixbuf.get_width(), pixbuf.get_height());
    //Making sure everything is valid
    if (hei * wid) * 3 != bytes.len() as i32 {
        return;
    }

    let row_stride = pixbuf.get_rowstride();

    unsafe {
        let raw_pixels = pixbuf.get_pixels();
        bytes
            .iter()
            .enumerate()
            .map(|(i, p)| (((i as i32) % (wid * 3), (i as i32) / (wid*3)), p))
            .map(|((x, y), p)| (y * row_stride + x, p))
            .for_each(|(i, p)| {
                raw_pixels[i as usize] = *p;
            });
    }
}

/*
let win = Window {
    p: (0.2934847027862066,-0.02028183490813784),
    r: 9.68342166526386e-10,
};
*/
fn main() {
    let application =
        Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        let window = ApplicationWindow::new(app);
        window.set_title("First GTK+ Program");
        window.set_default_size(500, 500);


        let pixbuf = Pixbuf::new(Colorspace::Rgb, false, 8, 500, 500).unwrap();
        bytes_to_image_buffer(&mandelbrot(50, 32, Window::default(), 500, 500), &pixbuf);
        
        let img = Image::new();
        img.set_property_pixbuf(Some(&pixbuf));

        let scrolled_window = ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);

        
        window.connect_check_resize(|event|{
            println!("Resize");
            

        });

        scrolled_window.add(&img);
        window.add(&scrolled_window);

        let button = Button::new_with_label("Click me!");
        button.connect_clicked(|_| {
            println!("Clicked!");
        });
        window.add(&button);

        window.show_all();
    });

    application.run(&[]);
}
