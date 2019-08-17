#![feature(test)]

extern crate test;

use gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gio::prelude::*;
use gtk::prelude::*;
use gtk::Image;
use gtk::WidgetExt;
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
            .map(|(i, p)| (((i as i32) % (wid * 3), (i as i32) / (wid * 3)), p))
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
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::thread::JoinHandle;
use std::cell::RefCell;
use std::rc::Rc;

pub type SimpleImage = (Vec<u8>, (u32, u32));
pub type MandelbrotState = (Window, usize, (u32, u32));

pub struct MandelUpdate {
    pub master: Arc<Mutex<RefCell<SimpleImage>>>,
    thread: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    enqueue: Option<Sender<MandelbrotState>>,
}


impl MandelUpdate {
    pub fn new(img: SimpleImage) -> Self {
        MandelUpdate {
            master: Arc::new(Mutex::new(RefCell::new(img))),
            thread: None,
            running: Arc::new(AtomicBool::new(false)),
            enqueue: None,
        }
    }

    pub fn update(&mut self, state: MandelbrotState) {
        let running = self.running.load(Ordering::Relaxed);

        if running {
            match &self.enqueue {
                Some(send) => {
                    match send.send(state) {
                        Ok(_) => {}
                        Err(_) => self.enqueue = None,
                    }
                }
                None => {}
            }
        }
        else {
            //Clear out any old data that may linger.
            self.enqueue = None;
            self.thread = None;

            let (send, new_data) = channel();
            send.send(state).unwrap();
            self.enqueue = Some(send);

            self.running.store(true, Ordering::Acquire);

            let output = self.master.clone();
            let is_runnig = self.running.clone();
            
            self.thread = Some(thread::spawn(move || {
                while let Ok((window, iter, (w, h))) = new_data.try_recv() {
                    let img = mandelbrot(iter, 32, window, w, h);
                    let out = output.lock();
                    
                    //We update the main image
                    match out {
                        Ok(data) => {
                            data.replace((img, (w, h)));
                        },
                        Err(_) => break,
                    }
                }

                is_runnig.store(false, Ordering::Acquire);
            }));

        }
        
    }
}

fn main() {
    let application =
        Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        let window = ApplicationWindow::new(app);
        window.set_title("First GTK+ Program");
        window.set_default_size(500, 500);

        let pixbuf = Pixbuf::new(Colorspace::Rgb, false, 8, 1, 1).unwrap();
        bytes_to_image_buffer(&[0x00, 0x00, 0x00], &pixbuf);
        let mandel_update = Rc::new(RefCell::new(MandelUpdate::new((vec![0x00, 0x00, 0x00], (1, 1)))));
        
        //We extract the master image that updates over time.
        let master = {
            let mu = mandel_update.borrow();
            mu.master.clone()
        };

        let img = Image::new();
        img.set_property_pixbuf(Some(&pixbuf));

        let scrolled_window = ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);

        let swindcp = scrolled_window.clone();
        let imgcp = img.clone();

        let update = mandel_update.clone();
        window.connect_check_resize(move |event| {
            let (size, _) = swindcp.get_allocated_size();
            let mut mandel_update = update.borrow_mut();
            let pbuf = {
                let lock = master.lock().unwrap();
                let data = lock.borrow();
                let pixbuf = Pixbuf::new(Colorspace::Rgb, false, 8, 1, 1).unwrap();
                bytes_to_image_buffer(data.0.as_ref(), &pixbuf);
                pixbuf
            };
            let window = Window{
                p: (0., 0.),
                r: 1.,
            };

            mandel_update.update((window, 50, (size.width as u32, size.height as u32)));

            let pbuf = imgcp.get_pixbuf().unwrap();
            let newbuf = Pixbuf::new(Colorspace::Rgb, false, 8, size.width, size.height).unwrap();

            pbuf.scale(
                &newbuf,
                0,
                0,
                size.width,
                size.height,
                0.,
                0.,
                f64::from(size.width) / f64::from(pbuf.get_width()),
                f64::from(size.height) / f64::from(pbuf.get_height()),
                InterpType::Nearest,
            );
            imgcp.set_from_pixbuf(Some(&newbuf));
            println!("{:?}", size);
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
