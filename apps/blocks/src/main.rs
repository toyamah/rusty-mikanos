#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::f64::consts::PI;
use core::panic::PanicInfo;
use libm::{cos, round, sin};
use shared_lib::app_event::AppEventType;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::window::{Window, FLAG_NO_DRAW};
use shared_lib::{create_timer, println, read_event, TimerType};

const NUM_BLOCKS_X: i32 = 10;
const NUM_BLOCKS_Y: i32 = 5;
const BLOCK_WIDTH: i32 = 20;
const BLOCK_HEIGHT: i32 = 10;
const BAR_WIDTH: i32 = 30;
const BAR_HEIGHT: i32 = 5;
const BALL_RADIUS: i32 = 5;
const GAP_WIDTH: i32 = 30;
const GAP_HEIGHT: i32 = 30;
const GAP_BAR: i32 = 80;
const BAR_FLOAT: i32 = 10;

const CANVAS_WIDTH: i32 = NUM_BLOCKS_X * BLOCK_WIDTH + 2 * GAP_WIDTH;
const CANVAS_HEIGHT: i32 =
    GAP_HEIGHT + NUM_BLOCKS_Y * BLOCK_HEIGHT + GAP_BAR + BAR_HEIGHT + BAR_FLOAT;
const BAR_Y: i32 = CANVAS_HEIGHT - BAR_FLOAT - BAR_HEIGHT;

const FRAME_RATE: i32 = 60; // frames/sec
const BAR_SPEED: i32 = CANVAS_WIDTH / 2; // pixels/sec
const BALL_SPEED: i32 = BAR_SPEED;

type Blocks = [[bool; NUM_BLOCKS_X as usize]; NUM_BLOCKS_Y as usize];

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut w = match Window::open((CANVAS_WIDTH, CANVAS_HEIGHT), (10, 10), "blocks") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    let mut blocks = [[false; NUM_BLOCKS_X as usize]; NUM_BLOCKS_Y as usize];

    for y in 0..NUM_BLOCKS_Y as usize {
        blocks[y].fill(true);
    }

    let ball_x = CANVAS_WIDTH / 2 - BALL_RADIUS - 20;
    let ball_y = CANVAS_HEIGHT - BAR_FLOAT - BAR_HEIGHT - BALL_RADIUS - 20;

    let mut bar_x = CANVAS_WIDTH / 2 - BAR_WIDTH / 2;
    let mut ball_x = ball_x;
    let mut ball_y = ball_y;
    let mut move_dir = 0; // -1: left, 1: right
    let mut ball_dir = 0; // degree
    let mut ball_dx = 0;
    let mut ball_dy = 0;

    'outer: loop {
        // 画面を一旦クリアし，各種オブジェクトを描画
        w.fill_rectangle((4, 24), (CANVAS_WIDTH, CANVAS_HEIGHT), 0, FLAG_NO_DRAW);
        draw_blocks(&mut w, &blocks);
        draw_bar(&mut w, bar_x);
        if ball_y >= 0 {
            draw_ball(&mut w, ball_x, ball_y);
        }
        w.draw();

        let mut prev_timeout = 0;
        if prev_timeout == 0 {
            let timeout =
                create_timer(TimerType::OneshotRel, 1, (1000 / FRAME_RATE) as u64).unwrap();
            prev_timeout = timeout;
        } else {
            prev_timeout += 1000 / FRAME_RATE as u64;
            create_timer(TimerType::OneshotAbs, 1, prev_timeout);
        }

        let mut events = [Default::default(); 1];
        loop {
            match read_event(events.as_mut(), 1) {
                Ok(_) => {}
                Err(e) => println!("ReadEvent failed: {}", e.strerror()),
            };

            let event = &events[0];
            match event.type_ {
                AppEventType::TimerTimeout => break,
                AppEventType::Quit => break 'outer,
                AppEventType::KeyPush => {
                    let arg = unsafe { event.arg.key_push };
                    if !arg.press {
                        move_dir = 0;
                    } else {
                        let keycode = arg.keycode;
                        if keycode == 79 {
                            move_dir = 1;
                        } else if keycode == 80 {
                            move_dir = -1;
                        } else if keycode == 44 {
                            if ball_dir == 0 && ball_y < 0 {
                                ball_x = ball_x;
                                ball_y = ball_y;
                            } else if ball_dir == 0 {
                                ball_dir = 45;
                            }
                        }
                        if bar_x == 0 && move_dir < 0 {
                            move_dir = 0;
                        } else if bar_x + BAR_WIDTH == CANVAS_WIDTH - 1 && move_dir > 0 {
                            move_dir = 0;
                        }
                    }
                }
                _ => {}
            }
        }

        bar_x += move_dir * BAR_SPEED / FRAME_RATE;
        bar_x = limit_range(bar_x, 0, CANVAS_WIDTH - BAR_WIDTH - 1);

        if ball_dir == 0 {
            continue;
        }

        let ball_x_ = ball_x + ball_dx;
        let ball_y_ = ball_y + ball_dy;
        if (ball_dx < 0 && ball_x_ < BALL_RADIUS)
            || (ball_dx > 0 && CANVAS_WIDTH - BALL_RADIUS <= ball_x_)
        {
            // 壁
            ball_dir = 180 - ball_dir;
        }
        if ball_dy < 0 && ball_y_ < BALL_RADIUS {
            // 天井
            ball_dir = -ball_dir;
        } else if bar_x <= ball_x_
            && ball_x_ < bar_x + BAR_WIDTH
            && ball_dy > 0
            && BAR_Y - BALL_RADIUS <= ball_y_
        {
            // バー
            ball_dir = -ball_dir;
        } else if ball_dy > 0 && CANVAS_HEIGHT - BALL_RADIUS <= ball_y_ {
            // 落下
            ball_dir = 0;
            ball_y = -1;
            continue;
        }

        loop {
            if ball_x_ < GAP_WIDTH
                || CANVAS_WIDTH - GAP_WIDTH <= ball_x_
                || ball_y_ < GAP_HEIGHT
                || GAP_HEIGHT + NUM_BLOCKS_Y * BLOCK_HEIGHT <= ball_y_
            {
                break;
            }

            let index_x = (ball_x_ - GAP_WIDTH) / BLOCK_WIDTH;
            let index_y = (ball_y_ - GAP_HEIGHT) / BLOCK_HEIGHT;
            if !blocks[index_y as usize][index_x as usize] {
                // ブロックが無い
                break;
            }

            // ブロックがある
            blocks[index_y as usize][index_x as usize] = false;

            let block_left = GAP_WIDTH + index_x * BLOCK_WIDTH;
            let block_right = GAP_WIDTH + (index_x + 1) * BLOCK_WIDTH;
            let block_top = GAP_HEIGHT + index_y * BLOCK_HEIGHT;
            let block_bottom = GAP_HEIGHT + (index_y + 1) * BLOCK_HEIGHT;
            if (ball_x < block_left && block_left <= ball_x_)
                || (block_right < ball_x && ball_x_ <= block_right)
            {
                ball_dir = 180 - ball_dir;
            }
            if (ball_y < block_top && block_top <= ball_y_)
                || (block_bottom < ball_y && ball_y_ <= block_bottom)
            {
                ball_dir = -ball_dir;
            }

            break;
        }

        let ball_speed = BALL_SPEED as f64;
        let frame_rate = FRAME_RATE as f64;
        ball_dx = round(ball_speed * cos(PI * ball_dir as f64 / 180.0) / frame_rate) as i32;
        ball_dy = round(ball_speed * sin(PI * ball_dir as f64 / 180.0) / frame_rate) as i32;
        ball_x += ball_dx;
        ball_y += ball_dy;
    }

    w.close();
    exit(0)
}

fn draw_blocks(w: &mut Window, blocks: &Blocks) {
    for by in 0..NUM_BLOCKS_Y {
        let y = 24 + GAP_HEIGHT + by * BLOCK_HEIGHT;
        let color: u32 = 0xff << (by % 3) * 8;

        for bx in 0..NUM_BLOCKS_X {
            if blocks[by as usize][bx as usize] {
                let x = 4 + GAP_WIDTH + bx * BLOCK_WIDTH;
                let c = color | (0xff << ((bx + by) % 3) * 8);
                w.fill_rectangle((x, y), (BLOCK_WIDTH, BLOCK_HEIGHT), c, FLAG_NO_DRAW);
            }
        }
    }
}

fn draw_ball(w: &mut Window, x: i32, y: i32) {
    w.fill_rectangle(
        (4 + x - BALL_RADIUS, 24 + y - BALL_RADIUS),
        (2 * BALL_RADIUS, 2 * BALL_RADIUS),
        0x007f00,
        FLAG_NO_DRAW,
    );

    w.fill_rectangle(
        (4 + x - BALL_RADIUS / 2, 24 + y - BALL_RADIUS / 2),
        (BALL_RADIUS, BALL_RADIUS),
        0x00ff00,
        FLAG_NO_DRAW,
    );
}

fn draw_bar(w: &mut Window, bar_x: i32) {
    w.fill_rectangle(
        (4 + bar_x, 24 + BAR_Y),
        (BAR_WIDTH, BAR_HEIGHT),
        0xffffff,
        FLAG_NO_DRAW,
    );
}

fn limit_range(x: i32, min: i32, max: i32) -> i32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
