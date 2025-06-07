// 行列の次元情報
struct MatrixDimensions {
    width: u32,
    height: u32,
}

// バインディング定義
@group(0) @binding(0) var<storage, read> matrix_a: array<f32>;
@group(0) @binding(1) var<storage, read> matrix_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> result: array<f32>;
@group(0) @binding(3) var<uniform> dimensions: MatrixDimensions;

// ワークグループサイズの定義
const WORKGROUP_SIZE: u32 = 16u;

// 共有メモリ（ローカルメモリ）でのタイル最適化
var<workgroup> tile_a: array<array<f32, WORKGROUP_SIZE>, WORKGROUP_SIZE>;
var<workgroup> tile_b: array<array<f32, WORKGROUP_SIZE>, WORKGROUP_SIZE>;

// WebGPU互換性を重視した単純版（デバッグ用）
@compute @workgroup_size(16, 16, 1)
fn main_simple(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.y;
    let col = global_id.x;
    let matrix_size = dimensions.width;
    
    // 境界チェック - early returnを避ける
    if row >= dimensions.height || col >= dimensions.width {
        return;
    }

    var sum = 0.0;
    
    // 標準的な行列乗算（同期処理なし）
    for (var k = 0u; k < matrix_size; k = k + 1u) {
        sum = sum + matrix_a[row * matrix_size + k] * matrix_b[k * matrix_size + col];
    }

    result[row * matrix_size + col] = sum;
}
