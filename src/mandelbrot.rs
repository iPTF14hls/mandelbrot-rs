
use num::Complex;
use std::num::NonZeroUsize;
use std::thread;

#[derive(Clone)]
pub struct Mandelbrot {
    iters: usize,
}

fn color_gradient(x: f64, colors: &[[u8; 3]]) -> Vec<u8> {
    let x = x * (colors.len() - 1) as f64;
    
    //Returns 
    if (x.floor() - x).abs() < 0.000_001 {
        let color = colors[x as usize];
        return vec![color[0], color[1], color[2]];
    }

    let x_prev = x.floor() as usize;
    let x_next = x.ceil() as usize;
    let x = x - x.floor();

    let c1 = colors[x_prev];
    let c2 = colors[x_next];

    let r = transfer(x, f64::from(c1[0]), f64::from(c2[0]));
    let g = transfer(x, f64::from(c1[1]), f64::from(c2[1]));
    let b = transfer(x, f64::from(c1[2]), f64::from(c2[2]));

    vec![r.round() as u8, g.round() as u8, b.round() as u8]
}


/*
We have a List of colors here
        [0x00, 0x00, 0xff],
        [0x00, 0xff, 0xff],
        [0x00, 0xff, 0x00],
        [0xff, 0xff, 0x00],
        [0xff, 0x00, 0x00],
        Smooth Color transitions

        [0x00, 0x00, 0x3f],
        [0x00, 0x90, 0x90],
        [0x00, 0xc0, 0x00],
        [0xe3, 0xff, 0x00],
        [0xff, 0x00, 0x00],
        Same as above but with dimming

 */
fn visualiser(c: Option<f64>) -> Vec<u8> {
    let colors = &[
        [0x00, 0x00, 0xff],
        [0x00, 0xff, 0xff],
        [0x00, 0xff, 0x00],
        [0xff, 0xff, 0x00],
        [0xff, 0x00, 0x00],
    ];
    match c {
        Some(a) => {
            color_gradient(a, colors)
        },
        None => vec![0x00, 0x00, 0x00],
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Window {
    pub p: (f64, f64),
    pub r: f64,
}

pub enum Aspect {
    Default,
    InAccordenceWith((u32, u32)),
}

impl Window {
    pub fn to_points(&self, aspect: Aspect) -> ((f64, f64), (f64, f64)) {
        let r = self.r * 2.;

        let (sx, sy) = match aspect {
            Aspect::Default => (1., 1.),
            Aspect::InAccordenceWith((w, h)) => {
                let (larg, smal, swap) = if w > h {
                    (f64::from(w), f64::from(h), false)
                }
                else {
                    (f64::from(h), f64::from(w), true)
                };

                let s = smal / larg;

                if swap {
                    (s, 1.)
                }
                else {
                    (1., s)
                }
            }
        };

        ((self.p.0-(r*sx), self.p.1-(r*sy)), (self.p.0+(r*sx), self.p.1+(r*sy)))
    }
}

impl Default for Window {
    fn default() -> Window {
        Window {
            p: (0., 0.),
            r: 1.
        }
    }
}

pub fn contains(iters: usize, c: &Complex<f64>) -> Option<NonZeroUsize> {
    let f = |x: Complex<f64>| -> Complex<f64> { x.powi(2) + c };
    let mut cx = Complex::new(0., 0.);

    //There will be at least 1 iteration
    for i in 0..iters {
        cx = f(cx);
        if cx.re.powf(2.) + cx.im.powf(2.) > 4. {
            return NonZeroUsize::new(i + 1);
        }
    }

    None
}

pub fn mandelbrot(iters: usize, threads: usize, win: Window, wid: u32, hei: u32) -> Vec<u8> {
    let max = u64::from(wid)*u64::from(hei);
    let mut ranges = Vec::with_capacity(threads);
    
    for i in 1..=threads {
        let j = i-1;
        let (i, j) = ((i as f64 / threads as f64), (j as f64 / threads as f64));
        let (i, j) = (transfer(i, 0., max as f64), transfer(j, 0., max as f64));
        let (i, j) = (i.round() as u32, j.round() as u32);
        ranges.push((j, i));
    }

    let threads: Vec<thread::JoinHandle<Vec<u8>>> = ranges.into_iter()
        .map(|range|{
            std::thread::spawn(move ||{
                transfer_to_complex(range.0..range.1, win, wid, hei)
                    .map(|c| contains(iters, &c))
                    .map(|c| c.map(|d|(d.get() as f64)/(iters as f64)))
                    .map(visualiser)
                    .flatten()
                    .collect()
            })
        })
        .collect();

    threads.into_iter()
        .map(|t|t.join().expect("Couldn't join thread"))
        .flatten()
        .collect()
}
fn transfer_to_complex(iter: std::ops::Range<u32>, win: Window, wid: u32, hei: u32) -> impl Iterator<Item=Complex<f64>> {
    let (p1, p2) = win.to_points(Aspect::InAccordenceWith((wid, hei)));
    iter
        .map(move |i| (i % wid, i / wid))
        .map(move |(x, y)| (f64::from(x) / f64::from(wid), f64::from(y) / f64::from(hei)))
        .map(move |(x, y)| (transfer(x, p1.0, p2.0), transfer(y, p1.1, p2.1)))
        .map(|(x, y)| Complex::new(x, y))
}

fn transfer(x: f64, a: f64, b: f64) -> f64 {
    x * (b - a) + a
}

#[cfg(test)]
mod tests {
    use test::{Bencher, black_box};
    use crate::{Window, mandelbrot};
    #[bench]
    fn bench_threads_1(b: &mut Bencher) {
        b.iter(|| {
            black_box(mandelbrot(50, 1, Window::default(), 1_000, 1_000));
        });
    }
    #[bench]
    fn bench_threads_2(b: &mut Bencher) {
        b.iter(|| {
            black_box(mandelbrot(50, 2, Window::default(), 1_000, 1_000));
        });
    }
    #[bench]
    fn bench_threads_4(b: &mut Bencher) {
        b.iter(|| {
            black_box(mandelbrot(50, 4, Window::default(), 1_000, 1_000));
        });
    }
    #[bench]
    fn bench_threads_8(b: &mut Bencher) {
        b.iter(|| {
            black_box(mandelbrot(50, 8, Window::default(), 1_000, 1_000));
        });
    }
    #[bench]
    fn bench_threads_16(b: &mut Bencher) {
        b.iter(|| {
            black_box(mandelbrot(50, 16, Window::default(), 1_000, 1_000));
        });
    }
    #[bench]
    fn bench_threads_32(b: &mut Bencher) {
        b.iter(|| {
            black_box(mandelbrot(50, 32, Window::default(), 1_000, 1_000));
        });
    }

    
}
