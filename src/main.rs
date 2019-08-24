#![feature(test)]
extern crate gtk;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;
extern crate test;

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use crate::mandelbrot::{mandelbrot, Window};

mod mandelbrot;
mod gtk_gui;

/*
let win = Window {
    p: (0.2934847027862066,-0.02028183490813784),
    r: 9.68342166526386e-10,
};
*/

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
                Some(send) => match send.send(state) {
                    Ok(_) => {}
                    Err(_) => self.enqueue = None,
                },
                None => {}
            }
        } else {
            //Clear out any old data that may linger.
            self.enqueue = None;
            self.thread = None;

            let (send, new_data) = channel();
            send.send(state).unwrap();
            self.enqueue = Some(send);

            self.running.store(true, Ordering::Relaxed);

            let output = self.master.clone();
            let is_runnig = self.running.clone();
            self.thread = Some(thread::spawn(move || {
                //We don't care about all the old stuff that came in.
                //We just need this done ASAP.
                while let Some((window, iter, (w, h))) = new_data.try_iter().last() {
                    let img = mandelbrot(iter, 32, window, w, h);
                    let out = output.lock();
                    //We update the main image
                    match out {
                        Ok(data) => {
                            println!("Updating…{}, {}", w, h);
                            data.replace((img, (w, h)));
                        }
                        Err(_) => break,
                    }
                }
                println!("Broke.");
                is_runnig.store(false, Ordering::Relaxed);
            }));
        }
    }
}

use crate::gtk_gui::Win;
use relm::Widget;

fn main() {
    Win::run(()).unwrap();
}
