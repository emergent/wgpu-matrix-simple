// 行列のサイズを定義
const MATRIX_SIZE: usize = 16;
const WORKGROUP_SIZE: u32 = 16; // コンピュートシェーダーのワークグループサイズ
use std::borrow::Cow;
use wgpu::util::DeviceExt;

// GPU用の行列データ構造
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MatrixDimensions {
    width: u32,
    height: u32,
    _padding: [u32; 2], // パディングでアライメント調整
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // WGPU インスタンスの作成
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::BROWSER_WEBGPU, // PRIMARYでも良い
        ..Default::default()
    });

    // アダプターの取得
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // デバイスとキューの作成
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Failed to create device");

    // コンピュートシェーダーの作成
    let shader =
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Matrix Multiply Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                include_str!("matrix_multiply.wgsl"),
            )),
        });

    // コンピュートパイプラインの作成
    let compute_pipeline = device.create_compute_pipeline(
        &wgpu::ComputePipelineDescriptor {
            label: Some("Matrix Multiply Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main_simple"),
            compilation_options: Default::default(),
            cache: None,
        },
    );

    ////// new ここまで

    let matrix_a_vec = create_random_matrix(MATRIX_SIZE);
    let matrix_b_vec = create_random_matrix(MATRIX_SIZE);
    let matrix_a = matrix_a_vec.as_slice();
    let matrix_b = matrix_b_vec.as_slice();
    let size = MATRIX_SIZE;

    //////

    let matrix_size_bytes =
        (size * size * std::mem::size_of::<f32>()) as u64;
    let dimensions = MatrixDimensions {
        width: size as u32,
        height: size as u32,
        _padding: [0, 0],
    };

    // バッファーの作成
    let buffer_a =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Matrix A Buffer"),
            contents: bytemuck::cast_slice(matrix_a),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let buffer_b =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Matrix B Buffer"),
            contents: bytemuck::cast_slice(matrix_b),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let buffer_result = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Result Buffer"),
        size: matrix_size_bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let buffer_dimensions =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dimensions Buffer"),
            contents: bytemuck::cast_slice(&[dimensions]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

    // バインドグループの作成
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Matrix Multiply Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_result.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_dimensions.as_entire_binding(),
                },
            ],
        });

    // ステージングバッファー（結果読み取り用）
    let staging_buffer =
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: matrix_size_bytes,
            usage: wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

    // コマンドエンコーダーの作成
    let mut encoder = device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor {
            label: Some("Matrix Multiply Encoder"),
        },
    );

    // コンピュートパスの実行
    {
        let mut compute_pass =
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Matrix Multiply Pass"),
                timestamp_writes: None,
            });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroups_x =
            (size as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let workgroups_y =
            (size as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    // 結果をステージングバッファーにコピー
    encoder.copy_buffer_to_buffer(
        &buffer_result,
        0,
        &staging_buffer,
        0,
        matrix_size_bytes,
    );

    // コマンドの実行
    queue.submit(Some(encoder.finish()));

    // 結果の読み取り
    let buffer_slice = staging_buffer.slice(..);

    // PC環境でのバッファー読み取り
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
        sender.send(v).ok();
    });

    device.poll(wgpu::PollType::Wait)?;
    receiver.await.unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    // 結果の表示
    println!("Matrix A:");
    for i in 0..size {
        for j in 0..size {
            print!("{:8.4} ", matrix_a[i * size + j]);
        }
        println!();
    }
    println!("\nMatrix B:");
    for i in 0..size {
        for j in 0..size {
            print!("{:8.4} ", matrix_b[i * size + j]);
        }
        println!();
    }
    println!("\nResult Matrix:");
    for i in 0..size {
        for j in 0..size {
            print!("{:8.4} ", result[i * size + j]);
        }
        println!();
    }

    Ok(())
}

pub fn create_random_matrix(size: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut matrix = Vec::with_capacity(size * size);
    for i in 0..size * size {
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let hash = hasher.finish();
        // 簡単な疑似乱数生成（0.0-1.0の範囲）
        matrix.push((hash % 1000) as f32 / 1000.0);
    }
    matrix
}
