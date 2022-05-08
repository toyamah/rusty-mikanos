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

const kNumBlocksX: i32 = 10;
const kNumBlocksY: i32 = 5;
const kBlockWidth: i32 = 20;
const kBlockHeight: i32 = 10;
const kBarWidth: i32 = 30;
const kBarHeight: i32 = 5;
const kBallRadius: i32 = 5;
const kGapWidth: i32 = 30;
const kGapHeight: i32 = 30;
const kGapBar: i32 = 80;
const kBarFloat: i32 = 10;

const kCanvasWidth: i32 = kNumBlocksX * kBlockWidth + 2 * kGapWidth;
const kCanvasHeight: i32 =
    kGapHeight + kNumBlocksY * kBlockHeight + kGapBar + kBarHeight + kBarFloat;
const kBarY: i32 = kCanvasHeight - kBarFloat - kBarHeight;

const kFrameRate: i32 = 60; // frames/sec
const kBarSpeed: i32 = kCanvasWidth / 2; // pixels/sec
const kBallSpeed: i32 = kBarSpeed;

type Blocks = [[bool; kNumBlocksX as usize]; kNumBlocksY as usize];

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut w = match Window::open((kCanvasWidth, kCanvasHeight), (10, 10), "blocks") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    let mut blocks = [[false; kNumBlocksX as usize]; kNumBlocksY as usize];

    for y in 0..kNumBlocksY as usize {
        blocks[y].fill(true);
    }

    let kBallX = kCanvasWidth / 2 - kBallRadius - 20;
    let kBallY = kCanvasHeight - kBarFloat - kBarHeight - kBallRadius - 20;

    let mut bar_x = kCanvasWidth / 2 - kBarWidth / 2;
    let mut ball_x = kBallX;
    let mut ball_y = kBallY;
    let mut move_dir = 0; // -1: left, 1: right
    let mut ball_dir = 0; // degree
    let mut ball_dx = 0;
    let mut ball_dy = 0;

    'outer: loop {
        // 画面を一旦クリアし，各種オブジェクトを描画
        w.fill_rectangle((4, 24), (kCanvasWidth, kCanvasHeight), 0, FLAG_NO_DRAW);
        draw_blocks(&mut w, &blocks);
        draw_bar(&mut w, bar_x);
        if ball_y >= 0 {
            draw_ball(&mut w, ball_x, ball_y);
        }
        w.draw();

        let mut prev_timeout = 0;
        if prev_timeout == 0 {
            let timeout =
                create_timer(TimerType::OneshotRel, 1, (1000 / kFrameRate) as u64).unwrap();
            prev_timeout = timeout;
        } else {
            prev_timeout += 1000 / kFrameRate as u64;
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
                                ball_x = kBallX;
                                ball_y = kBallY;
                            } else if ball_dir == 0 {
                                ball_dir = 45;
                            }
                        }
                        if bar_x == 0 && move_dir < 0 {
                            move_dir = 0;
                        } else if bar_x + kBarWidth == kCanvasWidth - 1 && move_dir > 0 {
                            move_dir = 0;
                        }
                    }
                }
                _ => {}
            }
        }

        bar_x += move_dir * kBarSpeed / kFrameRate;
        bar_x = limit_range(bar_x, 0, kCanvasWidth - kBarWidth - 1);

        if ball_dir == 0 {
            continue;
        }

        let ball_x_ = ball_x + ball_dx;
        let ball_y_ = ball_y + ball_dy;
        if (ball_dx < 0 && ball_x_ < kBallRadius)
            || (ball_dx > 0 && kCanvasWidth - kBallRadius <= ball_x_)
        {
            // 壁
            ball_dir = 180 - ball_dir;
        }
        if ball_dy < 0 && ball_y_ < kBallRadius {
            // 天井
            ball_dir = -ball_dir;
        } else if bar_x <= ball_x_
            && ball_x_ < bar_x + kBarWidth
            && ball_dy > 0
            && kBarY - kBallRadius <= ball_y_
        {
            // バー
            ball_dir = -ball_dir;
        } else if ball_dy > 0 && kCanvasHeight - kBallRadius <= ball_y_ {
            // 落下
            ball_dir = 0;
            ball_y = -1;
            continue;
        }

        loop {
            if ball_x_ < kGapWidth
                || kCanvasWidth - kGapWidth <= ball_x_
                || ball_y_ < kGapHeight
                || kGapHeight + kNumBlocksY * kBlockHeight <= ball_y_
            {
                break;
            }

            let index_x = (ball_x_ - kGapWidth) / kBlockWidth;
            let index_y = (ball_y_ - kGapHeight) / kBlockHeight;
            if !blocks[index_y as usize][index_x as usize] {
                // ブロックが無い
                break;
            }

            // ブロックがある
            blocks[index_y as usize][index_x as usize] = false;

            let block_left = kGapWidth + index_x * kBlockWidth;
            let block_right = kGapWidth + (index_x + 1) * kBlockWidth;
            let block_top = kGapHeight + index_y * kBlockHeight;
            let block_bottom = kGapHeight + (index_y + 1) * kBlockHeight;
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

        let ball_speed = kBallSpeed as f64;
        let frame_rate = kFrameRate as f64;
        ball_dx = round(ball_speed * cos(PI * ball_dir as f64 / 180.0) / frame_rate) as i32;
        ball_dy = round(ball_speed * sin(PI * ball_dir as f64 / 180.0) / frame_rate) as i32;
        ball_x += ball_dx;
        ball_y += ball_dy;
    }

    w.close();
    exit(0)
}

fn draw_blocks(w: &mut Window, blocks: &Blocks) {
    for by in 0..kNumBlocksY {
        let y = 24 + kGapHeight + by * kBlockHeight;
        let color: u32 = 0xff << (by % 3) * 8;

        for bx in 0..kNumBlocksX {
            if blocks[by as usize][bx as usize] {
                let x = 4 + kGapWidth + bx * kBlockWidth;
                let c = color | (0xff << ((bx + by) % 3) * 8);
                w.fill_rectangle((x, y), (kBlockWidth, kBlockHeight), c, FLAG_NO_DRAW);
            }
        }
    }
}

fn draw_ball(w: &mut Window, x: i32, y: i32) {
    w.fill_rectangle(
        (4 + x - kBallRadius, 24 + y - kBallRadius),
        (2 * kBallRadius, 2 * kBallRadius),
        0x007f00,
        FLAG_NO_DRAW,
    );

    w.fill_rectangle(
        (4 + x - kBallRadius / 2, 24 + y - kBallRadius / 2),
        (kBallRadius, kBallRadius),
        0x00ff00,
        FLAG_NO_DRAW,
    );
}

fn draw_bar(w: &mut Window, bar_x: i32) {
    w.fill_rectangle(
        (4 + bar_x, 24 + kBarY),
        (kBarWidth, kBarHeight),
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
