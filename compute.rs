use wgpu_bootstrap::{
    window::Window,
    frame::Frame,
    cgmath::{ self },
    application::Application,
    texture::create_texture_bind_group,
    context::Context,
    camera::Camera,
    default::{ Vertex, Particle },
    geometry::{icosphere, compute_line_list},
    computation::Computation,
    wgpu,
};

// Particule
const PARTICLE_RADIUS:f32 = 0.8;
const NUM_INSTANCES_PER_ROW: u32 = 15; 
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 1.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 1.5);
// Sphere
const SPHERE_RADIUS : f32 = 15.0; 
const SPHERE_X : f32 = 0.0;
const SPHERE_Y : f32 = 0.0;
const SPHERE_Z : f32 = 0.0; 
// Forces
const MASS: f32 = 1.0;
const REST_LENGTH: f32 = 6.0;
const STRUCTURAL_STIFFNESS: f32 = 1.0;
const SHEAR_STIFFNESS: f32 = 1.0;
const BEND_STIFFNESS: f32 = 1.0;
const DAMPING_FACTOR: f32 = 3.5;
// Friction
const FRICTION_FACTOR: f32 = 0.1;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeData {
    delta_time: f32,
    // Particles
    nb_instances: u32,
    particle_radius:f32,
    // Sphere
    sphere_x:f32,
    sphere_y:f32,
    sphere_z:f32,
    sphere_r:f32,
    // Forces
    mass:f32,
    rest_length:f32,
    structural_stiffness:f32,
    shear_stiffness:f32,
    bend_stiffness:f32,
    damping_factor:f32, 
    // Friction
    friction_factor:f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Neighbours { 
    pub particule_index: u32,
    pub neighbours: [f32;4],
}

fn calculate_neighbors(index :u32) -> [u32; 4] {
    let mut neighbours: [u32;4] = [8000; 4];
    let x = index % NUM_INSTANCES_PER_ROW;
    let z = index / NUM_INSTANCES_PER_ROW;
    if x > 0 {
        neighbours[0] = index - 1;
    }
    if x < NUM_INSTANCES_PER_ROW - 1 {
        neighbours[1] = index + 1;
    }
    if z > 0 {
        neighbours[2] = index - NUM_INSTANCES_PER_ROW;
    }
    if z < NUM_INSTANCES_PER_ROW - 1 {
        neighbours[3] = index + NUM_INSTANCES_PER_ROW;
    }
    // if index % 4 != 0 {
    //     neighbours[0] = index - 1;
    // }
    // if (index + 1) % 4 != 0 {
    //     neighbours[1] = index + 1;
    // }
    // if index >= 4 {
    //     neighbours[2] = index - 4;
    // }
    // if index <= 15 {
    //     neighbours[3] = index + 4;
    // }
    // for i in 0..(NUM_INSTANCES_PER_ROW* NUM_INSTANCES_PER_ROW) {
    //     if i == index {
    //         continue;
    //     }
    //     let x_i = i % NUM_INSTANCES_PER_ROW;
    //     let z_i = i / NUM_INSTANCES_PER_ROW;
    //     let distance = ((((x as i32) - (x_i as i32)) as f32).powi(2) + (((z as i32) - (z_i as i32)) as f32).powi(2)).sqrt();
    //     // println!("distance: {}", distance);
    //     if distance <= 4.0 {
    //         if x_i < x as u32 && neighbours[0] == 8000 {
    //             neighbours[0] = i;
    //         } else if x_i > x as u32 && neighbours[1] == 8000 {
    //             neighbours[1] = i;
    //         } else if z_i < z as u32 && neighbours[2] == 8000 {
    //             neighbours[2] = i;
    //         } else if z_i > z as u32 && neighbours[3] == 8000 {
    //             neighbours[3] = i;
    //         }
    //     }
    // }
    neighbours
}

struct MyApp {
    diffuse_bind_group: wgpu::BindGroup,
    camera_bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    compute_instances_bind_group: wgpu::BindGroup,
    compute_data_buffer: wgpu::Buffer,
    compute_data_bind_group: wgpu::BindGroup,
    // Particle
    particle_pipeline: wgpu::RenderPipeline,
    particle_vertex_buffer: wgpu::Buffer,
    particle_index_buffer: wgpu::Buffer,
    particles: Vec<Particle>,
    particle_buffer: wgpu::Buffer,
    nb_indices: usize,
    // Sphere
    sphere_pipeline: wgpu::RenderPipeline,
    sphere_vertex_buffer: wgpu::Buffer,
    sphere_index_buffer: wgpu::Buffer,
    sphere_nb_indices:usize,    
    // Neighbours
    // neighbours_buffer: wgpu::Buffer,
    // compute_neighbours_bind_group: wgpu::BindGroup,
}

impl MyApp {
    fn new(context: &Context) -> Self {
        
        // Texture
        let texture = context.create_srgb_texture("happy-tree.png", include_bytes!("happy-tree.png"));
        let diffuse_bind_group = create_texture_bind_group(context, &texture);
        
        // Camera
        let camera = Camera {
            eye: (40.0, 30.0, 80.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: context.get_aspect_ratio(),
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
        };
        let (_camera_buffer, camera_bind_group) = camera.create_camera_bind_group(context);

        // Sphere
        let sphere_pipeline = context.create_render_pipeline(
            "Render Pipeline",
            include_str!("blue.wgsl"),
            &[Vertex::desc()],
            &[
                &context.camera_bind_group_layout,
            ],
            wgpu::PrimitiveTopology::LineList
        );
        let (mut sphere_vertices, sphere_indices) = icosphere(4);

        // Change sphere r :
        for vertex in sphere_vertices.iter_mut() {
            let mut posn = cgmath::Vector3::from(vertex.position);
            posn *= SPHERE_RADIUS as f32;
            vertex.position = posn.into()
        }
        // Change sphere center:
        for vertex in sphere_vertices.iter_mut() {
            vertex.position[0] += SPHERE_X;
            vertex.position[1] += SPHERE_Y;
            vertex.position[2] += SPHERE_Z;
        }

        let sphere_indices = compute_line_list(sphere_indices);
        let sphere_vertex_buffer = context.create_buffer(sphere_vertices.as_slice(), wgpu::BufferUsages::VERTEX);
        let sphere_index_buffer = context.create_buffer(sphere_indices.as_slice(), wgpu::BufferUsages::INDEX);

        // Particles
        let particle_pipeline = context.create_render_pipeline(
            "Render Pipeline",
            include_str!("particle.wgsl"),
            &[Vertex::desc(), Particle::desc()],
            &[
                &context.texture_bind_group_layout,
                &context.camera_bind_group_layout,
            ],
            wgpu::PrimitiveTopology::TriangleList
        );
        let (mut particle_vertices, particle_indices) = icosphere(4);
       
        // Change particle radius
        for vertex in particle_vertices.iter_mut() {
            let mut posn = cgmath::Vector3::from(vertex.position);
            posn *= PARTICLE_RADIUS as f32;
            vertex.position = posn.into()
        }
        // Particle buffer
        let particle_vertex_buffer = context.create_buffer(particle_vertices.as_slice(), wgpu::BufferUsages::VERTEX);
        let particle_index_buffer = context.create_buffer(particle_indices.as_slice(), wgpu::BufferUsages::INDEX);

        let particles = (0..NUM_INSTANCES_PER_ROW*NUM_INSTANCES_PER_ROW).map(|index| { 
            let x = index % NUM_INSTANCES_PER_ROW;
            let z = index / NUM_INSTANCES_PER_ROW;
            let position = cgmath::Vector3 { x: x as f32 * 3.0, y: 25.0, z: z as f32 * 3.0 } - INSTANCE_DISPLACEMENT;
            let neighbours = calculate_neighbors(index); 
            // println!(" index : {}" , index);
            // println!("{}, {}, {}" , position.x, position.y, position.z);
            Particle {
                position: position.into(), velocity:[0.0,0.0,0.0], neighbors: neighbours,
            }
        }).collect::<Vec<_>>();
        
        let particle_buffer = context.create_buffer(particles.as_slice(), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE);
        let compute_pipeline = context.create_compute_pipeline("Compute Pipeline", include_str!("compute.wgsl"));
        let compute_instances_bind_group = context.create_bind_group(
            "Compute Bind Group",
            &compute_pipeline.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                }
            ]
        );
        
        let compute_data = ComputeData {
            delta_time: 0.016,
            nb_instances: NUM_INSTANCES_PER_ROW*NUM_INSTANCES_PER_ROW,
            particle_radius:PARTICLE_RADIUS,
            sphere_x: SPHERE_X,
            sphere_y: SPHERE_Y,
            sphere_z: SPHERE_Z,
            sphere_r: SPHERE_RADIUS,
            mass: MASS,
            rest_length: REST_LENGTH,
            structural_stiffness: STRUCTURAL_STIFFNESS,
            shear_stiffness: SHEAR_STIFFNESS,
            bend_stiffness: BEND_STIFFNESS,
            damping_factor: DAMPING_FACTOR,
            friction_factor: FRICTION_FACTOR,
        };

        let compute_data_buffer = context.create_buffer(&[compute_data], wgpu::BufferUsages::UNIFORM);
        let compute_data_bind_group = context.create_bind_group(
            "Compute Data", 
            &compute_pipeline.get_bind_group_layout(1), 
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: compute_data_buffer.as_entire_binding(),
                }
            ]
        );

        // Neighbours 
        // let mut neighbours: Vec<Neighbours> = Vec::new();
        // for i in (0..NUM_INSTANCES_PER_ROW*NUM_INSTANCES_PER_ROW).step_by(1) {
        //     let temp_neighbours = calculate_neighbors(i); 
        //     neighbours.push(Neighbours {
        //         particule_index: i as u32,
        //         neighbour_left: temp_neighbours.0,
        //         neighbour_right: temp_neighbours.1,
        //         neighbour_top: temp_neighbours.2,
        //         neighbour_bottom: temp_neighbours.3,
        //     });
        // }   

        // let neighbours_buffer = context.create_buffer(neighbours.as_slice(), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE);
        
        // let compute_neighbours_bind_group = context.create_bind_group(
        //     "Compute Bind Group",
        //     &compute_pipeline.get_bind_group_layout(2),
        //     &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: neighbours_buffer.as_entire_binding(),
        //         }
        //     ]
        // );

        Self {
            diffuse_bind_group,
            camera_bind_group,
            particle_pipeline,
            sphere_pipeline,
            compute_pipeline,
            particle_vertex_buffer,
            particle_index_buffer,
            sphere_vertex_buffer,
            sphere_index_buffer,
            particles,
            particle_buffer,
            compute_instances_bind_group,
            compute_data_buffer,
            compute_data_bind_group,
            nb_indices: particle_indices.len(),
            sphere_nb_indices: sphere_indices.len(),
        }
    }
}

impl Application for MyApp {
    fn render(&self, context: &Context) -> Result<(), wgpu::SurfaceError> {
        let mut frame = Frame::new(context)?;
        {
            let mut render_pass = frame.begin_render_pass(wgpu::Color {r: 0.1, g: 0.2, b: 0.3, a: 1.0});

            // Particule
            render_pass.set_pipeline(&self.particle_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.particle_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.particle_buffer.slice(..));
            render_pass.set_index_buffer(self.particle_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..(self.nb_indices as u32), 0, 0..self.particles.len() as _);
            
            // Sphere
            render_pass.set_pipeline(&self.sphere_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sphere_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.sphere_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..(self.sphere_nb_indices as u32), 0, 0..1);
        }
        frame.present();
        Ok(())
    }

    fn update(&mut self, context: &Context, delta_time: f32) {
        let compute_data = ComputeData {
            delta_time,
            nb_instances: NUM_INSTANCES_PER_ROW*NUM_INSTANCES_PER_ROW,
            particle_radius:PARTICLE_RADIUS, 
            sphere_x: SPHERE_X,
            sphere_y: SPHERE_Y,
            sphere_z: SPHERE_Z,
            sphere_r:SPHERE_RADIUS,
            mass: MASS,
            rest_length: REST_LENGTH,
            structural_stiffness: STRUCTURAL_STIFFNESS,
            shear_stiffness: SHEAR_STIFFNESS,
            bend_stiffness: BEND_STIFFNESS,
            damping_factor: DAMPING_FACTOR,
            friction_factor: FRICTION_FACTOR,
        }; 
        context.update_buffer(&self.compute_data_buffer, &[compute_data]);
        let mut computation = Computation::new(context);
        {
            let mut compute_pass = computation.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_instances_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_data_bind_group, &[]);
            compute_pass.dispatch_workgroups(((NUM_INSTANCES_PER_ROW * NUM_INSTANCES_PER_ROW) as f64/128.0).ceil() as u32, 1, 1);
        }
        computation.submit();
    }
}

fn main() {
    let window = Window::new();

    let context = window.get_context();

    let my_app = MyApp::new(context);

    window.run(my_app);
}
