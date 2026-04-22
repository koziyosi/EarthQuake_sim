mod engine;

use macroquad::prelude::*;
use macroquad::audio::{load_sound, play_sound_once};
use serde::Deserialize;
use engine::{Engine, FaultPlane};

fn conf() -> Conf {
    Conf {
        window_title: "Earthquake Simulation Pro (FHD)".to_owned(),
        window_width: 1920,
        window_height: 1080,
        fullscreen: false,
        ..Default::default()
    }
}

const LON_SCALE: f32 = 150.0;
const LAT_SCALE: f32 = 180.0;
const MAP_CENTER_LAT_INIT: f64 = 36.0;
const MAP_CENTER_LON_INIT: f64 = 138.0;

fn lat_lon_to_scratch(lat: f64, lon: f64, center_lat: f64, center_lon: f64, zoom: f32) -> Vec2 {
    let dx = (lon - center_lon) as f32 * LON_SCALE * zoom; // FHD用にスケール調整
    let dy = (lat - center_lat) as f32 * LAT_SCALE * zoom;
    Vec2::new(dx, dy)
}

fn scratch_to_screen(sx: f32, sy: f32) -> Vec2 {
    let x = sx + screen_width() / 2.0;
    let y = screen_height() / 2.0 - sy;
    Vec2::new(x, y)
}

fn get_shindo_color(intensity: f64) -> Color {
    if intensity < 0.5 { return Color::from_rgba(40, 40, 60, 255); }
    if intensity < 1.5 { return Color::from_rgba(0, 150, 255, 255); }
    if intensity < 2.5 { return Color::from_rgba(0, 255, 100, 255); }
    if intensity < 3.5 { return Color::from_rgba(255, 255, 0, 255); }
    if intensity < 4.5 { return Color::from_rgba(255, 120, 0, 255); }
    if intensity < 5.5 { return Color::from_rgba(255, 0, 0, 255); }
    if intensity < 6.5 { return Color::from_rgba(150, 0, 0, 255); }
    Color::from_rgba(255, 0, 255, 255)
}

fn handle_input(cam_lat: &mut f64, cam_lon: &mut f64, zoom: &mut f32, fault: &mut FaultPlane, start_time: &mut f64, chime: &macroquad::audio::Sound) -> bool {
    let mut fault_changed = false;
    if is_mouse_button_down(MouseButton::Right) {
        let delta = mouse_delta_position();
        *cam_lon -= (delta.x * (screen_width() / (LON_SCALE * *zoom))) as f64;
        *cam_lat += (delta.y * (screen_height() / (LAT_SCALE * *zoom))) as f64;
    }

    let wheel = mouse_wheel();
    if wheel.1 != 0.0 {
        let mouse_pos = mouse_position();
        let sw = screen_width();
        let sh = screen_height();

        let mx = (mouse_pos.0 - sw / 2.0) / *zoom as f32;
        let my = (sh / 2.0 - mouse_pos.1) / *zoom as f32;
        let lon_before = mx / LON_SCALE + *cam_lon as f32;
        let lat_before = my / LAT_SCALE + *cam_lat as f32;

        let zoom_factor = 1.05f32.powf(wheel.1.clamp(-3.0, 3.0));
        *zoom *= zoom_factor;
        *zoom = zoom.clamp(0.1, 100.0);

        let mx_after = (mouse_pos.0 - sw / 2.0) / *zoom as f32;
        let my_after = (sh / 2.0 - mouse_pos.1) / *zoom as f32;
        let lon_after = mx_after / LON_SCALE + *cam_lon as f32;
        let lat_after = my_after / LAT_SCALE + *cam_lat as f32;

        *cam_lon += (lon_before - lon_after) as f64;
        *cam_lat += (lat_before - lat_after) as f64;
    }

    if is_key_pressed(KeyCode::Space) {
        *start_time = get_time();
        play_sound_once(chime);
    }

    let old_mw = fault.mw;
    let old_strike = fault.strike;
    let old_dip = fault.dip;

    if is_key_down(KeyCode::Up) { fault.mw += 0.01; }
    if is_key_down(KeyCode::Down) { fault.mw -= 0.01; }
    if is_key_down(KeyCode::Left) { fault.strike -= 1.0; }
    if is_key_down(KeyCode::Right) { fault.strike += 1.0; }
    if is_key_down(KeyCode::W) { fault.dip += 0.5; }
    if is_key_down(KeyCode::S) { fault.dip -= 0.5; }

    fault.mw = fault.mw.clamp(1.0, 9.5);
    fault.dip = fault.dip.clamp(0.0, 90.0);

    if fault.mw != old_mw || fault.strike != old_strike || fault.dip != old_dip {
        fault_changed = true;
    }

    fault_changed
}

fn draw_coastline(coastline: &CoastlineLatLon, cam_lat: f64, cam_lon: f64, zoom: f32) {
    let coastline_color = Color::new(0.4, 0.5, 0.4, 1.0);
    let line_thickness = 2.0 * zoom.sqrt().clamp(0.5, 2.0);
    for segment in &coastline.0 {
        for i in 0..segment.len() - 1 {
            let p1 = segment[i];
            let p2 = segment[i + 1];

            let s1 = lat_lon_to_scratch(p1[0], p1[1], cam_lat, cam_lon, zoom);
            let pos1 = scratch_to_screen(s1.x, s1.y);
            let s2 = lat_lon_to_scratch(p2[0], p2[1], cam_lat, cam_lon, zoom);
            let pos2 = scratch_to_screen(s2.x, s2.y);

            draw_line(pos1.x, pos1.y, pos2.x, pos2.y, line_thickness, coastline_color);
        }
    }
}

#[derive(Clone)]
struct StationCache {
    p_time: f32,
    s_time: f32,
    intensity: f64,
}

fn update_station_cache(engine: &Engine, fault: &FaultPlane, cache: &mut Vec<StationCache>) {
    let (l, w) = engine::Engine::get_dimensions(fault.mw);
    for (i, p) in engine.points.iter().enumerate() {
        let dist = Engine::get_distance_to_fault(p.lat, p.lon, fault, l, w);
        let (p_time, s_time) = engine.get_travel_time(fault.depth, dist);
        let intensity = engine.calculate_intensity(fault, i, dist);

        cache[i] = StationCache {
            p_time,
            s_time,
            intensity,
        };
    }
}

fn draw_stations(engine: &Engine, cache: &[StationCache], cam_lat: f64, cam_lon: f64, zoom: f32, elapsed: f32, ui_tex: &Texture2D) {
    for (i, p) in engine.points.iter().enumerate() {
        let c = &cache[i];

        let mut color = Color::from_rgba(60, 60, 80, 80);
        let mut size = 2.0;
        let mut glow = false;

        if elapsed >= c.s_time {
            color = get_shindo_color(c.intensity);
            size = 6.0;
            glow = true;
        } else if elapsed >= c.p_time {
            color = Color::from_rgba(100, 100, 255, 180);
            size = 3.5;
        }

        let sc_pos = lat_lon_to_scratch(p.lat, p.lon, cam_lat, cam_lon, zoom);
        let pos = scratch_to_screen(sc_pos.x, sc_pos.y);

        if pos.x >= -50.0 && pos.x <= screen_width() + 50.0 && pos.y >= -50.0 && pos.y <= screen_height() + 50.0 {
            if glow {
                draw_circle(pos.x, pos.y, size * 1.8, Color::new(color.r, color.g, color.b, 0.3));

                let idx = match c.intensity {
                    i if i >= 6.5 => Some(8), // 7
                    i if i >= 6.0 => Some(7), // 6+
                    i if i >= 5.5 => Some(6), // 6-
                    i if i >= 5.0 => Some(5), // 5+
                    i if i >= 4.5 => Some(4), // 5-
                    i if i >= 3.5 => Some(3), // 4
                    i if i >= 2.5 => Some(2), // 3
                    i if i >= 1.5 => Some(1), // 2
                    i if i >= 0.5 => Some(0), // 1
                    _ => None
                };

                if let Some(id) = idx {
                    let row = id / 3;
                    let col = id % 3;
                    let src_rect = Rect::new(40.0 + col as f32 * 130.0, 40.0 + row as f32 * 130.0, 64.0, 64.0);
                    draw_texture_ex(ui_tex, pos.x - 12.0, pos.y - 12.0, WHITE, DrawTextureParams {
                        dest_size: Some(Vec2::new(24.0, 24.0)),
                        source: Some(src_rect),
                        ..Default::default()
                    });
                }
            } else {
                draw_circle(pos.x, pos.y, size, color);
            }
        }
    }
}

fn draw_epicenter(fault: &FaultPlane, cam_lat: f64, cam_lon: f64, zoom: f32) {
    let (l, _w) = engine::Engine::get_dimensions(fault.mw);
    let epi_sc = lat_lon_to_scratch(fault.lat, fault.lon, cam_lat, cam_lon, zoom);
    let epi_pos = scratch_to_screen(epi_sc.x, epi_sc.y);

    let l_px = l as f32 * 0.4 * zoom;

    draw_poly_lines(epi_pos.x, epi_pos.y, 4, l_px.max(5.0), fault.strike + 45.0, 2.0, WHITE);
}

fn draw_ui(fault: &FaultPlane, zoom: f32, elapsed: f32) {
    let (l, w) = engine::Engine::get_dimensions(fault.mw);
    let panel_width = 500.0;
    let panel_height = 280.0;
    draw_rectangle(20.0, 20.0, panel_width, panel_height, Color::from_rgba(0, 0, 0, 180));
    draw_rectangle_lines(20.0, 20.0, panel_width, panel_height, 2.0, Color::from_rgba(100, 100, 100, 255));

    draw_text("EARTHQUAKE FINITE FAULT SIM", 40.0, 60.0, 32.0, WHITE);
    draw_line(40.0, 75.0, 40.0 + panel_width - 40.0, 75.0, 1.0, GRAY);

    draw_text(&format!("Mw: {:.2}", fault.mw), 40.0, 110.0, 30.0, YELLOW);
    draw_text("Moment Magnitude", 180.0, 110.0, 20.0, LIGHTGRAY);

    draw_text(&format!("Fault: {:.1}km x {:.1}km", l, w), 40.0, 145.0, 25.0, WHITE);
    draw_text(&format!("Strike: {:.0}° / Dip: {:.0}°", fault.strike, fault.dip), 40.0, 175.0, 25.0, SKYBLUE);
    draw_text(&format!("Epicenter Depth: {:.1} km", fault.depth), 40.0, 205.0, 25.0, SKYBLUE);
    draw_text(&format!("Zoom Level: {:.2}x", zoom), 40.0, 230.0, 20.0, GRAY);

    let time_color = if elapsed > 0.0 { WHITE } else { GRAY };
    draw_text(&format!("Elapsed Time: {:.1}s", elapsed), 40.0, 265.0, 40.0, time_color);

    if elapsed > 0.0 && elapsed < 10.0 {
        draw_rectangle(0.0, 0.0, screen_width(), 60.0, Color::from_rgba(200, 0, 0, 200));
        draw_text("緊急地震速報 (警報)", screen_width() / 2.0 - 150.0, 42.0, 40.0, WHITE);
    }

    draw_text("Controls: Arrow keys (Mw/Strike), W/S (Dip), SPACE (Reset), Right-Drag (Pan), Wheel (Zoom)", 20.0, screen_height() - 20.0, 20.0, DARKGRAY);
}

#[derive(Deserialize)]
struct Coastline(Vec<Vec<[f32; 2]>>);

struct CoastlineLatLon(Vec<Vec<[f64; 2]>>);

#[macroquad::main(conf)]
async fn main() {
    let engine = Engine::new();
    // 資産のロード
    let ui_tex = load_texture("assets/ui_assets.png").await.expect("Failed to load UI assets");
    ui_tex.set_filter(FilterMode::Linear);
    let chime = load_sound("assets/chime_std.wav").await.expect("Failed to load chime");

    // 海岸線データのロードと緯度経度への事前変換
    let coastline_str = macroquad::file::load_string("assets/coastline.json").await.expect("Failed to load coastline");
    let coastline_raw: Coastline = serde_json::from_str(&coastline_str).unwrap_or(Coastline(vec![]));
    
    let mut coastline = CoastlineLatLon(vec![]);
    let scale_val = 0.240048;
    let sprite_x = -70.3868;
    let sprite_y = -10.2063;

    for segment in coastline_raw.0 {
        let mut new_segment = vec![];
        for p in segment {
            let stage_x = (p[0] - 240.0) * scale_val + sprite_x;
            let stage_y = (180.0 - p[1]) * scale_val + sprite_y;
            let lon = (stage_x / LON_SCALE + MAP_CENTER_LON_INIT as f32) as f64;
            let lat = (stage_y / LAT_SCALE + MAP_CENTER_LAT_INIT as f32) as f64;
            new_segment.push([lat, lon]);
        }
        coastline.0.push(new_segment);
    }
    
    let mut fault = FaultPlane {
        lat: 35.0,
        lon: 135.0,
        depth: 10.0,
        mw: 8.5,
        strike: 220.0,
        dip: 30.0,
    };

    let mut cam_lat = MAP_CENTER_LAT_INIT;
    let mut cam_lon = MAP_CENTER_LON_INIT;
    let mut zoom = 1.0f32;
    let mut start_time = get_time();

    let mut station_cache = vec![StationCache { p_time: 0.0, s_time: 0.0, intensity: 0.0 }; engine.points.len()];
    update_station_cache(&engine, &fault, &mut station_cache);

    loop {
        let fault_changed = handle_input(&mut cam_lat, &mut cam_lon, &mut zoom, &mut fault, &mut start_time, &chime);

        if fault_changed {
            update_station_cache(&engine, &fault, &mut station_cache);
        }

        let elapsed = (get_time() - start_time) as f32;

        clear_background(BLACK);

        draw_coastline(&coastline, cam_lat, cam_lon, zoom);
        draw_stations(&engine, &station_cache, cam_lat, cam_lon, zoom, elapsed, &ui_tex);
        draw_epicenter(&fault, cam_lat, cam_lon, zoom);
        draw_ui(&fault, zoom, elapsed);

        next_frame().await
    }
}
