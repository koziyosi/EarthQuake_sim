use macroquad::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct ObservationPoint {
    pub lat: f64,
    pub lon: f64,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub point_type: String,
}

#[derive(Deserialize)]
pub struct SimulationData {
    pub travel_time: Vec<f32>,
    pub arv: Vec<f32>,
}

pub struct FaultPlane {
    pub lat: f64,
    pub lon: f64,
    pub depth: f64,
    pub mw: f64,
    pub strike: f32, // 走向 (North = 0, clockwise)
    pub dip: f32,    // 傾斜 (0 = horizontal)
}

pub struct Engine {
    pub points: Vec<ObservationPoint>,
    pub sim_data: SimulationData,
}

const KM_PER_LAT_DEGREE: f64 = 111.19;
const KM_PER_LON_DEGREE_EQUATOR: f64 = 111.32;
const TRAVEL_TIME_TABLE_STRIDE: usize = 472;

impl Engine {
    pub fn new() -> Self {
        let points_str = include_str!("../assets/points.json");
        let points: Vec<ObservationPoint> = serde_json::from_str(points_str).unwrap();
        let sim_str = include_str!("../assets/simulation_data.json");
        let sim_data: SimulationData = serde_json::from_str(sim_str).unwrap();
        Self { points, sim_data }
    }

    // スケーリング法則 (Wells & Coppersmith)
    pub fn get_dimensions(mw: f64) -> (f64, f64) {
        let length = 10.0f64.powf(0.5 * mw - 1.88);
        let width = 10.0f64.powf(0.32 * mw - 1.01);
        (length.max(0.1), width.max(0.1))
    }

    // 断層面までの最短距離 (Drup)
    pub fn get_distance_to_fault(p_lat: f64, p_lon: f64, fault: &FaultPlane, l: f64, w: f64) -> f64 {
        // 座標変換: 地点(p)を断層中心を原点とする局所座標系(km)に変換
        let avg_lat = (p_lat + fault.lat) / 2.0;
        let dy = (p_lat - fault.lat) * KM_PER_LAT_DEGREE;
        let dx = (p_lon - fault.lon) * KM_PER_LON_DEGREE_EQUATOR * avg_lat.to_radians().cos();
        let dz = -fault.depth; // 深度 (z軸は上向き正とする)

        // Strike回転 (Z軸まわり)
        let s_rad = (fault.strike as f64).to_radians();
        let lx = dx * s_rad.cos() + dy * s_rad.sin();
        let ly = -dx * s_rad.sin() + dy * s_rad.cos();
        let lz = dz;

        // Dip回転 (X軸まわり)
        let d_rad = (fault.dip as f64).to_radians();
        let fx = lx;
        let fy = ly * d_rad.cos() + lz * d_rad.sin();
        let fz = -ly * d_rad.sin() + lz * d_rad.cos();

        // 矩形面(-L/2 to L/2, -W/2 to W/2, z=0)への最短距離
        let qx = fx.abs() - l / 2.0;
        let qy = fy.abs() - w / 2.0;
        let qz = fz.abs();

        let d_ext = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2) + qz.powi(2)).sqrt();
        
        // 面の内側（投影範囲内）にいる場合の補正
        if qx <= 0.0 && qy <= 0.0 {
            qz
        } else {
            d_ext
        }
    }

    pub fn get_travel_time(&self, depth: f64, dist: f64) -> (f32, f32) {
        let max_dist_idx = (TRAVEL_TIME_TABLE_STRIDE / 2) - 1; // 235
        let depth_idx = (depth / 10.0).round() as usize;
        let dist_idx = (dist.round() as usize).min(max_dist_idx);
        
        let base_idx = depth_idx * TRAVEL_TIME_TABLE_STRIDE;
        let p_idx = base_idx + dist_idx * 2;
        let s_idx = p_idx + 1;
        
        if s_idx < self.sim_data.travel_time.len() {
            (self.sim_data.travel_time[p_idx], self.sim_data.travel_time[s_idx])
        } else {
            // データ範囲外は線形近似 (P: 7km/s, S: 4km/s)
            (dist as f32 / 7.0, dist as f32 / 4.0)
        }
    }

    pub fn calculate_intensity(&self, fault: &FaultPlane, point_idx: usize, dist: f64) -> f64 {
        // 断層面までの最短距離を使用
        let d_rup = dist.max(1.0);
        let m = fault.mw;
        
        let term1 = 0.58 * m + 0.003 * fault.depth - 1.29;
        let term2 = (d_rup + 0.0028 * 10.0f64.powf(0.5 * m)).log10();
        let term3 = 0.002 * d_rup;
        
        let pgv_base = 10.0f64.powf(term1 - term2 - term3);
        let arv = self.sim_data.arv.get(point_idx).cloned().unwrap_or(1.0) as f64;
        let pgv = pgv_base * arv;
        
        let intensity = 2.0 * pgv.log10() + 0.94;
        intensity.max(0.0)
    }
}
