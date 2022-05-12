#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![allow(unused_assignments)]
use core::arch::asm;
use core::cmp;
use core::f64::consts::PI;
use core::panic::PanicInfo;
use libm::{cos, sin};
use shared_lib::app_event::AppEventType;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::window::{Window, FLAG_NO_DRAW};
use shared_lib::{create_timer, println, read_event, TimerType};

const SCALE: i32 = 50;
const MARGIN: i32 = 10;
const CANVAS_SIZE: i32 = 3 * SCALE + MARGIN;

const CUBE: [(i32, i32, i32); 8] = [
    (1, 1, 1),
    (1, 1, -1),
    (1, -1, 1),
    (1, -1, -1),
    (-1, 1, 1),
    (-1, 1, -1),
    (-1, -1, 1),
    (-1, -1, -1),
];
const SURFACE: [[i32; 4]; 6] = [
    [0, 4, 6, 2],
    [1, 3, 7, 5],
    [0, 2, 3, 1],
    [0, 1, 5, 4],
    [4, 5, 7, 6],
    [6, 7, 3, 2],
];
const COLOR: [u32; SURFACE.len()] = [0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff];

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut w = match Window::open((CANVAS_SIZE, CANVAS_SIZE), (10, 10), "cube") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    let mut vert = [(0.0, 0.0, 0.0); CUBE.len()];
    let mut centerz4 = [0.0; SURFACE.len()];
    let mut scr = [(0, 0); CUBE.len()];

    let mut thx = 0;
    let mut thy = 0;
    let mut thz = 0;
    let to_rad = PI / 0x8000 as f64;
    loop {
        // 立方体を X, Y, Z 軸回りに回転
        thx = (thx + 182) & 0xffff;
        thy = (thy + 273) & 0xffff;
        thz = (thz + 364) & 0xffff;
        let xp = cos(thx as f64 * to_rad);
        let xa = sin(thx as f64 * to_rad);
        let yp = cos(thy as f64 * to_rad);
        let ya = sin(thy as f64 * to_rad);
        let zp = cos(thz as f64 * to_rad);
        let za = sin(thz as f64 * to_rad);

        let scale = SCALE as f64;
        for i in 0..CUBE.len() {
            let cv = CUBE[i];
            let cv = (cv.0 as f64, cv.1 as f64, cv.2 as f64);
            let zt = scale * cv.2 * xp + scale * cv.1 * xa;
            let yt = scale * cv.1 * xp - scale * cv.2 * xa;
            let xt = scale * cv.0 * yp + zt * ya;
            vert[i].2 = zt * yp - scale * cv.0 * ya;
            vert[i].0 = xt * zp - yt * za;
            vert[i].1 = yt * zp + xt * za;
        }

        // 面中心の Z 座標（を 4 倍した値）を 6 面について計算
        for sur in 0..SURFACE.len() {
            centerz4[sur] = 0.0;

            for i in 0..SURFACE[sur].len() {
                let pos = SURFACE[sur][i] as usize;
                centerz4[sur] += vert[pos].2;
            }
        }
        // 画面を一旦クリアし，立方体を描画
        w.fill_rectangle((4, 24), (CANVAS_SIZE, CANVAS_SIZE), 0, FLAG_NO_DRAW);
        draw_obj(&mut w, &mut vert, &mut scr, &mut centerz4);
        w.draw();
        if sleep(50) {
            break;
        }
    }

    w.close();
    exit(0)
}

fn draw_obj(
    w: &mut Window,
    vert: &mut [(f64, f64, f64)],
    scr: &mut [(i32, i32)],
    centerz4: &mut [f64],
) {
    // オブジェクト座標 vert を スクリーン座標 scr に変換（画面奥が Z+）
    let scale = SCALE as f64;
    let canvas_size = CANVAS_SIZE as f64;
    for i in 0..CUBE.len() {
        let t = 6.0 * scale / (vert[i].2 + 8.0 * scale);
        scr[i].0 = ((vert[i].0 * t) + canvas_size / 2.0) as i32;
        scr[i].1 = ((vert[i].1 * t) + canvas_size / 2.0) as i32;
    }
    loop {
        // 奥にある（= Z 座標が大きい）オブジェクトから順に描画
        let (max_i, &zmax) = centerz4
            .iter()
            .enumerate()
            .max_by(|x, y| x.1.partial_cmp(y.1).unwrap())
            .unwrap();
        if zmax == f64::MIN {
            break;
        }
        centerz4[max_i] = f64::MIN;

        // 法線ベクトルがこっちを向いてる面だけ描画
        let v0 = vert[SURFACE[max_i][0] as usize];
        let v1 = vert[SURFACE[max_i][1] as usize];
        let v2 = vert[SURFACE[max_i][2] as usize];
        let e0x = v1.0 - v0.0;
        let e0y = v1.1 - v0.1; // v0 --> v1
        let e1x = v2.0 - v1.0;
        let e1y = v2.1 - v1.1; // v1 --> v2
        if e0x * e1y <= e0y * e1x {
            draw_surface(w, max_i, scr);
        }
    }
}

fn draw_surface(w: &mut Window, sur: usize, scr: &mut [(i32, i32)]) {
    let surface = SURFACE[sur];

    // 画面の描画範囲 [ymin, ymax]
    let mut ymin = CANVAS_SIZE;
    let mut ymax = 0;

    // Y, X 座標の組
    let mut y2x_up = [0_i32; CANVAS_SIZE as usize];
    let mut y2x_down = [0_i32; CANVAS_SIZE as usize];

    for i in 0..surface.len() {
        let p0 = scr[surface[(i + 3) % 4] as usize];
        let p1 = scr[surface[i] as usize];
        ymin = cmp::min(ymin, p1.1);
        ymax = cmp::max(ymax, p1.1);
        if p0.1 == p1.1 {
            continue;
        }

        let mut y2x = &mut y2x_up;
        let mut x0: i32 = 0;
        let mut y0: i32 = 0;
        let mut y1: i32 = 0;
        let mut dx: i32 = 0;
        if p0.1 < p1.1 {
            // p0 --> p1 は上る方向
            y2x = &mut y2x_up;
            x0 = p0.0;
            y0 = p0.1;
            y1 = p1.1;
            dx = p1.0 - p0.0;
        } else {
            // p0 --> p1 は下る方向
            y2x = &mut y2x_down;
            x0 = p1.0;
            y0 = p1.1;
            y1 = p0.1;
            dx = p0.0 - p1.0;
        }
        let m = dx as f64 / (y1 - y0) as f64;
        let roundish = if dx >= 0 { libm::floor } else { libm::ceil };
        for y in y0..=y1 {
            y2x[y as usize] = roundish(m * (y - y0) as f64 + x0 as f64) as i32;
        }
    }
    for y in ymin..=ymax {
        let i = y as usize;
        let p0x = cmp::min(y2x_up[i], y2x_down[i]);
        let p1x = cmp::max(y2x_up[i], y2x_down[i]);
        w.fill_rectangle(
            (4 + p0x, 24 + y),
            (p1x - p0x + 1, 1),
            COLOR[sur],
            FLAG_NO_DRAW,
        );
    }
}

fn sleep(ms: u64) -> bool {
    let mut prev_timeout = 0;
    if prev_timeout == 0 {
        let timeout = create_timer(TimerType::OneshotRel, 1, ms).unwrap();
        prev_timeout = timeout;
    } else {
        prev_timeout += ms;
        create_timer(TimerType::OneshotAbs, 1, prev_timeout).unwrap();
    }

    let mut events = [Default::default(); 1];
    loop {
        match read_event(events.as_mut(), 1) {
            Ok(_) => {}
            Err(e) => {
                println!("ReadEvent failed: {}", e.strerror());
            }
        };

        let event = &events[0];
        match event.type_ {
            AppEventType::Quit => return true,
            AppEventType::TimerTimeout => return false,
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
