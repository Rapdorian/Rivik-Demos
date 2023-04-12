use std::{
    mem,
    sync::{Arc, RwLock},
};

use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use rivik::{
    assets::{
        formats::{img::ImageFormat, mesh::ObjMesh},
        load,
    },
    egui,
    render::{
        draw::{mesh, pixel_mesh, Mesh, PixelMesh, SkyMesh},
        lights::{ambient::AmbientLight, sun::SunLight},
        load::{GpuMesh, GpuTexture},
        tracing::UiSubscriber,
        Transform,
    },
    scene::Node,
    winit::event::{ElementState, VirtualKeyCode, WindowEvent},
    Handle,
};
use tracing::{dispatcher::set_global_default, Dispatch};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

pub struct App {
    car: Handle<Mesh>,
    speed: f32,
    rotation: f32,
    positon: Vec4,

    cam_position: Vec3,

    color: &'static str,
    last_color: &'static str,

    // input flags
    gas: bool,
    brake: bool,
    left: bool,
    right: bool,
}

impl rivik::App for App {
    fn init(ctx: &mut rivik::Context) -> Self {
        //ctx.show_trace = true;
        load_track(ctx);
        ctx.insert_light(SunLight::new(Vec3::ONE, Vec3::new(2.0, 1.0, 0.0)));
        ctx.insert_light(AmbientLight::new(0.05, 0.05, 0.1));
        ctx.insert(load_sky("file:assets/sky.jpeg", ImageFormat::Jpeg));
        ctx.camera = Mat4::look_at_rh(
            Vec3::new(00.0, 10.0, 20.0),
            Vec3::new(0.0, 0.0, 00.0),
            Vec3::Y,
        );
        ctx.far = 10_000.0;

        Self {
            car: load_car(
                ctx,
                "file:assets/textures/CompactCar_Texture_Muscle_Red.png",
            ),
            speed: 0.0,
            rotation: 0.0,
            positon: Vec4::new(-6.8, 0.0, 17.0, 1.0),
            cam_position: Vec3::ZERO,

            color: "Neon",
            last_color: "Neon",

            brake: false,
            gas: false,
            left: false,
            right: false,
        }
    }

    // fn ui(&mut self, ctx: &egui::Context) {
    //     egui::Window::new("Car Selector").show(ctx, |ui| {
    //         egui::ComboBox::from_label("Color")
    //             .selected_text(format!("{}", self.color))
    //             .show_ui(ui, |ui| {
    //                 ui.selectable_value(&mut self.color, "Black", "Black");
    //                 ui.selectable_value(&mut self.color, "Blue", "Blue");
    //                 ui.selectable_value(&mut self.color, "Brown", "Brown");
    //                 ui.selectable_value(&mut self.color, "Gray", "Gray");
    //                 ui.selectable_value(&mut self.color, "Green", "Green");
    //                 ui.selectable_value(&mut self.color, "Muscle_Blue", "Muscle Blue");
    //                 ui.selectable_value(&mut self.color, "Muscle_Orange", "Muscle Orange");
    //                 ui.selectable_value(&mut self.color, "Muscle_Red", "Muscle Red");
    //                 ui.selectable_value(&mut self.color, "Neon", "Neon");
    //                 ui.selectable_value(&mut self.color, "Orange", "Orange");
    //                 ui.selectable_value(&mut self.color, "Pink", "Pink");
    //                 ui.selectable_value(&mut self.color, "Police", "Police");
    //                 ui.selectable_value(&mut self.color, "Red", "Red");
    //                 ui.selectable_value(&mut self.color, "Taxi", "Taxi");
    //                 ui.selectable_value(&mut self.color, "White", "White");
    //                 ui.selectable_value(&mut self.color, "Yellow", "Yellow");
    //             });
    //     });
    // }

    fn update(&mut self, ctx: &mut rivik::Context) {
        // car control
        const ACC: f32 = 0.05;
        const DECEL: f32 = 0.07;
        const STEER: f32 = 0.02;
        const MAX: f32 = 0.23;
        const RESIST: f32 = 0.02;

        if self.gas {
            self.speed += (MAX - self.speed) * ACC;
        } else {
            // slow down due to friction;
            self.speed -= self.speed * RESIST;
        }
        if self.brake {
            self.speed -= self.speed * DECEL;
        }

        self.speed = self.speed.max(0.0);

        // // TODO: Steering needs to be reworked as a sideways force
        let mut steer = 0.0;
        if self.left {
            steer += 1.0;
        }
        if self.right {
            steer -= 1.0;
        }

        // alternate steering is based off the left vector of the car
        // no energy is added to the car so we need to maintain the magnitude of the velocity
        // we then add a sideways force depending on the steer direction
        // then we need to restore the velocity's magnitude
        //
        // when displaying the car we need to compute a rotation from the velocity (this is an
        // issude when speed = 0)

        self.rotation += steer * STEER;

        let velocity = Vec4::new(
            self.rotation.sin() * self.speed,
            0.0,
            self.rotation.cos() * self.speed,
            0.0,
        );

        self.positon += velocity;

        let local_position = Mat4::from_translation(self.positon.xyz())
            * Mat4::from_rotation_y(-self.rotation)
            * Vec4::new(0.0, 0.0, 0.0, 1.0);

        // we have a car position

        self.car.transform(ctx).write().unwrap().update(
            Mat4::from_translation(local_position.xyz()) * Mat4::from_rotation_y(self.rotation),
        );

        // update camera position to be behind car
        let focus = self.positon.xyz();
        let eye = focus + Vec3::new(self.rotation.sin() * -2.0, 2.0, self.rotation.cos() * -2.0);

        let eye = self.cam_position.lerp(eye, 0.06);
        self.cam_position = eye;
        ctx.camera = Mat4::look_at_rh(eye, focus, Vec3::Y);
    }

    fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                // handle key inputs
                match input.virtual_keycode.unwrap_or(VirtualKeyCode::Kanji) {
                    VirtualKeyCode::W => self.gas = input.state == ElementState::Pressed,
                    VirtualKeyCode::S => self.brake = input.state == ElementState::Pressed,
                    VirtualKeyCode::A => self.left = input.state == ElementState::Pressed,
                    VirtualKeyCode::D => self.right = input.state == ElementState::Pressed,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn load_mesh(mesh: &str, tex: &str, fmt: ImageFormat) -> Mesh {
    let mesh = load(mesh, GpuMesh(ObjMesh, mesh::vertex_buffer)).unwrap();
    let tex = load(tex, GpuTexture(fmt)).unwrap();
    Mesh::new(mesh, tex)
}

fn load_sky(tex: &str, fmt: ImageFormat) -> SkyMesh {
    let mesh = load("file:assets/sky.obj", GpuMesh(ObjMesh, mesh::vertex_buffer)).unwrap();
    let tex = load(tex, GpuTexture(fmt)).unwrap();
    SkyMesh::new(mesh, tex)
}

fn load_car(ctx: &mut rivik::Context, texture: &str) -> Handle<Mesh> {
    let handle = ctx.insert(load_mesh("file:assets/car.obj", texture, ImageFormat::Png));

    let node = handle.transform(ctx).clone();
    let node = &mut *node.write().unwrap();

    ctx.insert_child(
        node,
        load_mesh("file:assets/wheel.obj", texture, ImageFormat::Png),
    )
    .transform(ctx)
    .write()
    .unwrap()
    .update(Mat4::from_translation(Vec3::new(
        0.587519, 0.300258, -1.08391,
    )));

    ctx.insert_child(
        node,
        load_mesh("file:assets/wheel.obj", texture, ImageFormat::Png),
    )
    .transform(ctx)
    .write()
    .unwrap()
    .update(Mat4::from_translation(Vec3::new(
        -0.587519, 0.300258, -1.08391,
    )));

    ctx.insert_child(
        node,
        load_mesh("file:assets/wheel.obj", texture, ImageFormat::Png),
    )
    .transform(ctx)
    .write()
    .unwrap()
    .update(Mat4::from_translation(Vec3::new(
        0.60354, 0.299993, 1.35941,
    )));

    ctx.insert_child(
        node,
        load_mesh("file:assets/wheel.obj", texture, ImageFormat::Png),
    )
    .transform(ctx)
    .write()
    .unwrap()
    .update(Mat4::from_translation(Vec3::new(
        -0.60354, 0.299993, 1.35941,
    )));
    handle
}

fn load_track(ctx: &mut rivik::Context) {
    ctx.insert(load_mesh(
        "file:assets/track.obj",
        "file:assets/textures/track.png",
        ImageFormat::Png,
    ));
    ctx.insert(load_mesh(
        "file:assets/advertisment.obj",
        "file:assets/textures/raid.jpg",
        ImageFormat::Jpeg,
    ));
    ctx.insert(load_mesh(
        "file:assets/billboard_base.obj",
        "file:assets/textures/track.png",
        ImageFormat::Png,
    ));
    ctx.insert(load_mesh(
        "file:assets/ground.obj",
        "file:assets/textures/track.png",
        ImageFormat::Png,
    ));
    ctx.insert(load_mesh(
        "file:assets/billboard_sign.obj",
        "file:assets/textures/flag.png",
        ImageFormat::Png,
    ));
}

fn main() {
    set_global_default(Dispatch::new(
        Registry::default().with(UiSubscriber::default()),
    ))
    .unwrap();
    rivik::run::<App>();
}
