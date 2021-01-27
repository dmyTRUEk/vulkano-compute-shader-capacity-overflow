use std::sync::Arc;

use vulkano::{
    instance::{
        Instance,
        InstanceExtensions,
        PhysicalDevice,
        // ApplicationInfo
    },
    device::{
        Device,
        DeviceExtensions,
        Features,
        RawDeviceExtensions,
        Queue,
    },
    buffer::{
        BufferUsage,
        CpuAccessibleBuffer,
    },
    command_buffer::{
        AutoCommandBufferBuilder,
        CommandBuffer,
    },
    sync,
    sync::GpuFuture,
    pipeline::{
        ComputePipeline,
        cache::PipelineCache,
    },
    descriptor::{
        descriptor_set::PersistentDescriptorSet,
        PipelineLayoutAbstract,
    }
};



mod compute_shader {
    vulkano_shaders::shader!{
        ty: "compute",
        src: "
            #version 450
            
            layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;
            
            layout(set = 0, binding = 0) buffer StructDataIn {
                uint data_in[];
            } buf_in;


            layout(set = 0, binding = 1) buffer StructDataOut {
                double data_out[];
            } buf_out;
            
            void main() {
                uint idx = gl_GlobalInvocationID.x;

                buf_in.data_in[idx] = idx;
                buf_out.data_out[idx] = double(idx);
            }
        ",
    }
}



fn make_calculations_on_gpu(data_in: Vec<u32>, device: Arc<Device>, queue: Arc<Queue>) -> Vec<f64> {
    let data_in_size = data_in.clone().len();
    println!("data_in_size = {}", data_in_size);
    let data_buffer_in = CpuAccessibleBuffer::from_data(
        device.clone(),
        BufferUsage::all(),
        false,
        data_in
    ).expect("failed to create buffer");

    let data_out: Vec<f64> = vec![0.0; data_in_size];
    let data_buffer_out = CpuAccessibleBuffer::from_data(
        device.clone(),
        BufferUsage::all(),
        false,
        data_out
    ).expect("failed to create buffer");



    let shader = compute_shader::Shader::load(device.clone())
        .expect("failed to create shader module");

    let compute_pipeline = Arc::new(
        ComputePipeline::new(
            device.clone(),
            &shader.main_entry_point(),
            &(),
            None
        ).expect("failed to create compute pipeline")
    );

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
            .add_buffer(data_buffer_in.clone()).unwrap()
            .add_buffer(data_buffer_out.clone()).unwrap()
            .build().unwrap()
    );

    let mut builder = AutoCommandBufferBuilder::new(
        device.clone(),
        queue.family()
    ).unwrap();

    builder.dispatch(
        [data_in_size as u32 / 64 + 1, 1, 1],
        compute_pipeline.clone(),
        set.clone(),
        (),
    ).unwrap();

    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap()
        .wait(None).unwrap();

    let content_in = data_buffer_in.read().unwrap();
    let content_out = data_buffer_out.read().unwrap();
    
    println!("in size: {:?}", content_in.len());
    println!("out size: {:?}", content_out.len());
    // println!("{:?}", content_in[0]);

    // let content_out = data_buffer_out.read().unwrap();
    return content_out.clone();

    // return vec![1.0, 2.0];
}



fn main() {
    println!("Program Started!");

    let instance = {
        let required_instance_extensions = InstanceExtensions {
            khr_surface: true,
            khr_display: true,
            khr_xlib_surface: true,
            khr_xcb_surface: true,
            khr_wayland_surface: true,
            ext_debug_utils: true,
            khr_get_physical_device_properties2: true,
            khr_get_surface_capabilities2: true,
            .. InstanceExtensions::none()
        };
        Instance::new(
            None,
            &required_instance_extensions,
            None
        ).expect("failed to create instance")
    };

    let physical_device = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device available");

    for ph_dev in PhysicalDevice::enumerate(&instance) {
        println!("Name: {}", ph_dev.name());
    }



    for family in physical_device.queue_families() {
        println!("Found a queue family with {:?} queue(s)", family.queues_count());
    }

    let queue_family = physical_device
        .queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    let (device, mut queues) = {
        let required_device_features = Features {
            robust_buffer_access: true,
            shader_float64: true,
            .. Features::none()
        };
        let required_device_extensions = DeviceExtensions {
            khr_storage_buffer_storage_class: true,
            .. DeviceExtensions::none()
        };
        Device::new(
            physical_device,
            &required_device_features,
            RawDeviceExtensions::from(&required_device_extensions),
            [(queue_family, 0.5)].iter().cloned()
        ).expect("failed to create device")
    };

    let queue = queues.next().unwrap();



    let res = make_calculations_on_gpu(vec![0, 1, 2, 3, 4, 5], device, queue);
    println!("{:?}", res);



    println!("Program Finished Successfully!");
}



