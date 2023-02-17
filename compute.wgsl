struct Particle {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    vel_x: f32,
    vel_y: f32,
    vel_z: f32,
    neighbours_left: u32,
    neighbours_right: u32,
    neighbours_top: u32,
    neighbours_bottom: u32,
}

struct ComputationData {
    delta_time: f32,
    nb_instances: u32,
    particle_radius: f32,
    sphere_x: f32,
    sphere_y: f32,
    sphere_z: f32,
    sphere_r: f32,
    mass: f32,
    structural_stiffness: f32,
    shear_stiffness: f32,
    bend_stiffness: f32,
    damping_factor: f32,
    rest_length: f32,
    friction_factor: f32,
}

// struct Neighbours { 
//     particule_index: u32,
//     neighbours_left: u32,
//     neighbours_right: u32,
//     neighbours_top: u32,
//     neighbours_bottom: u32,
// }

@group(0) @binding(0) var<storage, read_write> particlesData: array<Particle>;
@group(1) @binding(0) var<uniform> data: ComputationData;

fn calculateForces(particle: Particle, data: ComputationData) ->  vec3<f32> {
    let Fx = 0.0; 
    let Fy = -9.81 *data.mass ; 
    let Fz = 0.0;
    return vec3<f32>(Fx, Fy, Fz);
}

// fn getNeighbourPosition(neighbour: f32) -> vec3<f32> {
//     if (neighbour == 0.0) {
//         return vec3<f32>(0.0, 0.0, 0.0);
//     }
//     return vec3<f32>(particlesData[neighbour].pos_x, particlesData[neighbour].pos_y, particlesData[neighbour].pos_z);
// }

fn cross(a: vec3<f32> ,b: vec3<f32> ) -> vec3<f32> {
    return vec3(a.y*b.z - a.z*b.y, a.z*b.x - a.x*b.z, a.x*b.y - a.y*b.x);
}

@compute @workgroup_size(255, 1, 1) 
fn main(@builtin(global_invocation_id) param: vec3<u32>) {
    if (param.x >= u32(data.nb_instances)) {
          return;
    }
    // Particule
    var particle = particlesData[param.x];
    let position = vec3<f32>(particle.pos_x, particle.pos_y, particle.pos_z);
    let particle_radius = data.particle_radius;
    // Sphere
    let sphere_center = vec3<f32>(data.sphere_x, data.sphere_y, data.sphere_z);
    let sphere_radius = data.sphere_r;
    
    // Forces
    let neighbours = vec4(particlesData[param.x].neighbours_left, particlesData[param.x].neighbours_right, particlesData[param.x].neighbours_top, particlesData[param.x].neighbours_bottom);
    // Gravity
    let forces = calculateForces(particlesData[param.x], data);
    var Fx = forces.x; 
    var Fy = forces.y; 
    var Fz = forces.z;
    
    // Structural springs
    // let left_neighbourg_index = particlesData[param.x].neighbours_left; 
    // let left_neighbourg = particlesData[left_neighbourg_index]; 
    for (var i=0; i <= 4 ; i++) {
        var neighbour = neighbours[i]; 
        let v:u32 = 8000u;
        if (neighbour != v){
            let neighbour_position = vec3(particlesData[neighbour].pos_x, particlesData[neighbour].pos_y, particlesData[neighbour].pos_z);
            let distance = length(neighbour_position-position);
            let normal = normalize(neighbour_position - position);
            // if (!(isNan(normal.x))) {
            // }
            Fx += (data.structural_stiffness*(distance - data.rest_length)*normal.x);
            Fy += (data.structural_stiffness*(distance - data.rest_length)*normal.y);
            Fz += (data.structural_stiffness*(distance - data.rest_length)*normal.z);
        }
    } 

    // Damping
    Fx += -data.damping_factor * particle.vel_x;
    Fy += -data.damping_factor * particle.vel_y;
    Fz += -data.damping_factor * particle.vel_z;

    // Friction 
    let normal = normalize(sphere_center - position);
    let reference = vec3(1.0, 0.0, 0.0); // a reference vector that is not parallel to the normal vector
    let tangent = cross(reference, normal); 
    let resultantO = vec3(Fx,Fy,Fz);
    let resultantN = dot(resultantO, normal) * normal;
    let resultantT = resultantO - resultantN; 
    let Ff = -min(sqrt(dot(resultantT, resultantT)), data.friction_factor * sqrt(dot(resultantN, resultantN))) * normalize(resultantT);
    Fx += Ff.x;
    Fy += Ff.y;
    Fz += Ff.z;
    
    // Update 
    particlesData[param.x].vel_x = particlesData[param.x].vel_x + data.delta_time*(Fx/data.mass);
    particlesData[param.x].vel_y = particlesData[param.x].vel_y + data.delta_time*(Fy/data.mass);
    particlesData[param.x].vel_z = particlesData[param.x].vel_z + data.delta_time*(Fz/data.mass);

    particlesData[param.x].pos_x += data.delta_time * particle.vel_x;
    particlesData[param.x].pos_y += data.delta_time * particle.vel_y;
    particlesData[param.x].pos_z += data.delta_time * particle.vel_z;

    // Collision
    let distance = length(sphere_center - position) - sphere_radius - particle_radius;
    if (distance < 0.0) { 
        particlesData[param.x].pos_x += normal.x * distance;
        particlesData[param.x].pos_y += normal.y * distance;
        particlesData[param.x].pos_z += normal.z * distance;
        
        particlesData[param.x].vel_x = 0.0;
        particlesData[param.x].vel_y = 0.0;
        particlesData[param.x].vel_z = 0.0;
    } 
}
