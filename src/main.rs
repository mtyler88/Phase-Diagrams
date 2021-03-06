extern crate imageproc;
extern crate image;
extern crate nalgebra;
extern crate rayon;

use std::fs::File;
use std::path::Path;
use nalgebra::Vector2;
use std::f64::consts::PI;
use rayon::prelude::*;

use imageproc::drawing::*;

fn rk4_integrate(r: &Vector2<f64>, f: &Vec<Box<Fn(Vector2<f64>) -> f64>>, h: f64) -> Vector2<f64> {
    let k1 = Vector2::new(f[0](*r), f[1](*r));
    let k2 = Vector2::new(f[0](*r+(h/2.0)*k1), f[1](*r+(h/2.0)*k1));
    let k3 = Vector2::new(f[0](*r+(h/2.0)*k2), f[1](*r+(h/2.0)*k2));
    let k4 = Vector2::new(f[0](*r+h*k3), f[1](*r+h*k3));
    r+(h/6.0)*(k1+2.0*k2+2.0*k3+k4)
}

fn v2t(v: Vector2<f64>) -> (f32, f32) {(v[0] as f32*127.32+400.0, v[1] as f32*127.32+400.0)}
fn pendulum_qdot(r: Vector2<f64>) -> f64 { r[1] }
fn pendulum_pdot(r: Vector2<f64>) -> f64 { -r[0].sin() }
fn attractor_pdot(r: Vector2<f64>) -> f64 { -r[0].sin()-r[1] }
fn dissipate_qdot(r: Vector2<f64>) -> f64 { r[1] }
fn gen_dissipate_pdot(a: f64, w: f64) -> Box<Fn(Vector2<f64>) -> f64> { Box::new( move |r| -2.0*a*r[1] - w.powf(2.0)*r[0].sin()) }
fn map_val(x: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.1-b.0)/(a.1-a.0)*(x-a.0)+b.0 
}
fn map_point(x: Vector2<f64>, a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> (f32, f32) {
    (map_val(x[0], a.0, b.0) as f32, map_val(x[1], a.1, b.1) as f32)    
}

fn dist(p1: (f32, f32), p2: (f32, f32)) -> f32 {
    ((p2.0-p1.0).powf(2.0)+(p2.1-p1.1).powf(2.0)).sqrt()
}

struct PhasePos {
    pos: Vector2<f64>,
    cur_pos: Vector2<f64>,
    bounds: Option<(f64, f64)>,
    bounds_l: f64,
    vel: Vec<Box<Fn(Vector2<f64>) -> f64>>,
}

impl PhasePos {
    fn new(p: Vector2<f64>, v: Vec<Box<Fn(Vector2<f64>) -> f64>>) -> PhasePos {
        PhasePos{pos: p, bounds: None, bounds_l: 0.0, cur_pos: p, vel: v}
    }
    fn new_bounded(p: Vector2<f64>, b: (f64, f64), v: Vec<Box<Fn(Vector2<f64>) -> f64>>) -> PhasePos {
        PhasePos{pos: p, bounds: Some(b), bounds_l: b.1-b.0, cur_pos: p, vel: v}
    } 
    fn wrap(&mut self, x: f64) -> f64 {
        match self.bounds {
            Some(b) => {
                let mut y = x;
                loop {
                    if y<b.0 { y+=self.bounds_l; }
                    else if y>b.1 { y-=self.bounds_l; }
                    else { break; }
                }
                y
            },
            None => x,
        }
    }

    fn reset(&mut self) {
        self.cur_pos = self.pos;
    }

    fn new_pos(&mut self, p: Vector2<f64>) {
        self.pos = p;
        self.cur_pos = p;
    }
}

impl Iterator for PhasePos {
    type Item = Vector2<f64>;
    fn next(&mut self) -> Option<Vector2<f64>> {
        let mut new_pos = rk4_integrate(&self.cur_pos, &self.vel, 0.01);
        new_pos[0] = self.wrap(new_pos[0]);
        self.cur_pos = new_pos;
        Some(new_pos)
    }
}

fn main() {
    let imgx=800;
    let imgy=800;
    let n_line = 50;

    let mut img = image::DynamicImage::new_rgb8(imgx, imgy);
    (0..100).into_par_iter()
        .for_each(|frame| {
            let mut img = image::DynamicImage::new_rgb8(imgx, imgy);
            let ref mut fout = File::create(format!("phase_gif/phase.{:03}.png", frame)).unwrap();
            //Iterate through each line in the phase diagram
            for i in 0..n_line {
                // Find the p value of the point; all q values are the same
                // The p value changes at each frame
                let p = map_val(i as f64 + f64::from(frame)/100.0, (0.0, n_line as f64), (-12.0, 12.0));
                let system = PhasePos::new_bounded(Vector2::new(0.0, p), (-PI, PI), vec!(Box::new(pendulum_qdot), gen_dissipate_pdot((2.0*PI*f64::from(frame)/100.0).sin(), 0.5)));
                let raw_positions: Vec<Vector2<f64>> = system.take(20000).collect();
                // Calculate the colours for each line segment from the absolute value of the
                // momentum
                let colours: Vec<u8> = raw_positions.iter().map(|&p| (p[1].abs()/5.0 * 255.0) as u8).collect();
                let positions: Vec<(f32, f32)> = raw_positions.iter().map(
                    |&p|{map_point(p, ((-PI, PI), (-4.0, 4.0)), ((0.0, imgx as f64),(0.0, imgy as f64) ))} 
                    ).collect();
                let z_pos = positions.iter().zip(positions.iter().skip(1));

                for (i, (p1, p2)) in z_pos.enumerate() {
                    // If the phase diagram wraps around, don't draw the line
                    if dist(*p1, *p2) < 200.0 {
                        draw_line_segment_mut(&mut img, *p1, *p2, image::Rgba([colours[i], 0, 255-colours[i], 255]));
                    }
                }
            }
            img.save(fout, image::PNG);
        }
    )
}
