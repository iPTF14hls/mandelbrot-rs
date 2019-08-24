use relm::{Relm, Update, Widget, Channel};
use gtk::{Window, Inhibit, WindowType, Image, ScrolledWindow};
use gtk::{WidgetExt, ContainerExt, GtkWindowExt, ImageExt};
use gdk_pixbuf::{Pixbuf, Colorspace, InterpType};
use crate::mandelbrot;
use std::sync::mpsc::{channel, Sender};

pub struct UpdateInfo {
    iter: usize,
    dim: (u32, u32),
    window: mandelbrot::Window,
}

pub struct Model {
    img: Image,
    scrolled: ScrolledWindow,
    master: Pixbuf,
    _channel: Channel<(Vec<u8>, (i32, i32))>,
    update: Sender<UpdateInfo>
}

#[derive(Msg)]
pub enum Msg {
    Redraw,
    Recalc((Vec<u8>, (i32, i32))),
    Quit,
}

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

pub struct Win {
    model: Model,
    window: Window,
}

impl Win {
    fn resize_img(&mut self) {
        let size = self.model.scrolled.clone();
        let w = size.get_allocated_width();
        let h = size.get_allocated_height();
        
        let master = self.model.master.clone();
        let dw = master.get_width();
        let dh = master.get_height();
        
        let sw = f64::from(w) / f64::from(dw);
        let sh = f64::from(h) / f64::from(dh);

        let canvas = Pixbuf::new(Colorspace::Rgb, false, 8, w, h).unwrap();

        master.scale(&canvas, 0, 0, w, h, 0., 0., sw, sh, InterpType::Nearest);
        let img = self.model.img.clone();
        img.set_from_pixbuf(Some(&canvas));
    }
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    // Return the initial model.
    fn model(relm: &Relm<Self>, _: ()) -> Model {
        let master = Pixbuf::new(Colorspace::Rgb, false, 8, 1, 1).unwrap();
        bytes_to_image_buffer(&[0x00, 0x00, 0x00], &master);

        let stream = relm.stream().clone();

        let (_channel, sender) = Channel::new(move |data: (Vec<u8>, (i32, i32))| {
            stream.emit(Msg::Recalc(data));
        });

        let (update, incoming) = channel();

        std::thread::spawn(move || {
            loop {
                let render_data: UpdateInfo = incoming.recv().unwrap();

                //We need the freshist data in order to continue.
                let render_data = match incoming.try_iter().last() {
                    Some(data) => data,
                    None => render_data,
                };

                let (w, h) = render_data.dim;
                let data = mandelbrot::mandelbrot(render_data.iter, 32, render_data.window, w, h);

                sender.send((data, (w as i32, h as i32))).expect("Failed to send updated data");
            }
        });

        let scrolled = ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        let img = Image::new();
        scrolled.add(&img);

        Model {
            img,
            master,
            _channel,
            update,
            scrolled,
        }
    }

    // The model may be updated when a message is received.
    // Widgets may also be updated in this function.
    fn update(&mut self, event: Msg) {
        match event {
            Msg::Redraw => {
                self.resize_img();
                let scrolled = self.model.scrolled.clone();
                let w = scrolled.get_allocated_width();
                let h = scrolled.get_allocated_height();

                self.model.update.send(UpdateInfo{
                    iter: 50,
                    dim: (w as u32, h as u32),
                    window: mandelbrot::Window::default(),
                }).unwrap();
            }

            Msg::Recalc((data, (w, h))) => {
                let master = Pixbuf::new(Colorspace::Rgb, false, 8, w, h).unwrap();
                bytes_to_image_buffer(data.as_ref(), &master);
                self.model.master = master;

                self.resize_img();
            }
            Msg::Quit => gtk::main_quit(),
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Window;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    // Create the widgets.
    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        // GTK+ widgets are used normally within a `Widget`.
        let window = Window::new(WindowType::Toplevel);
        window.set_default_size(500, 500);
        window.set_title("mandelbrot-rs");

        model.update.send(UpdateInfo{
            iter: 50,
            dim: (500, 500),
            window: mandelbrot::Window::default(),
        }).unwrap();
        
        let scrolled_window = model.scrolled.clone();

        // Connect the signal `delete_event` to send the `Quit` message.
        connect!(relm, window, connect_delete_event(_, _), return (Some(Msg::Quit), Inhibit(false)));
        connect!(relm, scrolled_window, connect_draw(_, _), return (Some(Msg::Redraw), Inhibit(false)));
        // There is also a `connect!()` macro for GTK+ events that do not need a
        // value to be returned in the callback.

        window.add(&scrolled_window);

        window.show_all();

        Win {
            model,
            window,
        }
    }
}