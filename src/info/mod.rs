use crate::clml_rs;
use crate::regex;
use crate::sysinfo;

use crate::errors;
pub(crate) mod kernel;
pub(crate) mod user;
pub(crate) mod host;
pub(crate) mod distro;
pub(crate) mod uptime;
pub(crate) mod package_managers;
pub(crate) mod shell;
pub(crate) mod resolution;
pub(crate) mod wm;
pub(crate) mod de;
pub(crate) mod utils;
pub(crate) mod cpu;
pub(crate) mod gpu;

use std::fs;
use std::env;
use std::path::{ Path };

use clml_rs::{ CLML };
use regex::{ Regex };
use sysinfo::{ SystemExt };

use crate::{ Inject };
use kernel::{ Kernel };
use user::{ User };
use host::{ Host };
use distro::{ Distro };
use uptime::{ Uptime };
use package_managers::{ PackageManagers };
use shell::{ Shell };
use resolution::{ Resolution };
use wm::{ Wm };
use de::{ De };
use utils::{ get_system };
use cpu::{ Cpu };
use gpu::{ Gpus };

pub(crate) struct Info {
	ctx: CLML,
	rendered: String,
	width: i32,
	height: i32,
	user: User,
	host: Host,
	pub distro: Distro,
	pub kernel: Kernel,
	pub uptime: Uptime,
	pub package_managers: PackageManagers,
	pub shell: Shell,
	pub resolution: Option<Resolution>,
	pub de: Option<De>,
	pub wm: Option<Wm>,
	pub cpu: Option<Cpu>,
    pub gpu: Option<Gpus>,
}

impl Info {
	pub fn new() -> Self {
		get_system().refresh_all();
		let kernel = Kernel::new();
		let distro = Distro::new(&kernel);
		let uptime = Uptime::new(&kernel);
		let package_managers = PackageManagers::new(&kernel);
		let shell = Shell::new(&kernel);
		let resolution = Resolution::new();
		let de = De::new(&kernel, &distro);
		let wm = Wm::new(&kernel);
		let cpu = Cpu::new(&kernel);
        let gpu = Gpus::new(&kernel);
		dbg!(&gpu);
        Info {
			ctx: CLML::new(),
			rendered: String::new(),
			width: 0,
			height: 0,
			user: User::new(),
			host: Host::new(),
			distro: distro,
			kernel: kernel,
			uptime: uptime,
			package_managers: package_managers,
			shell: shell,
			resolution: resolution,
			de: de,
			wm: wm,
			cpu: cpu,
            gpu: gpu,
		}
	}
	pub fn render(&mut self) -> Result<(), ()> {
		let info = Path::new("/home/")
			.join(env::var("USER").unwrap_or(String::new()))
			.join(".config/freshfetch/info.clml");
		if info.exists() {
			match fs::read_to_string(&info) {
				Ok(file) => {
					self.rendered = self.ctx
						.parse(&file)
						.or(Err(()))?;
				}
				Err(e) => {
					errors::handle(&format!("{}{file:?}{}{err}",
						errors::io::READ.0,
						errors::io::READ.1,
						file = info,
						err = e));
					panic!();
				}
			}
		} else {
			self.rendered = self.ctx
				.parse(include_str!("../assets/defaults/info_wip.clml"))
				.or(Err(()))?;
		}
		Ok(())
	}
}

impl Inject for Info {
	fn prep(&mut self) -> Result<(), ()> {
		self.user.inject(&mut self.ctx)?;
		self.host.inject(&mut self.ctx)?;
		self.kernel.inject(&mut self.ctx)?;
		self.distro.inject(&mut self.ctx)?;
		self.uptime.inject(&mut self.ctx)?;
		self.package_managers.inject(&mut self.ctx)?;
		self.shell.inject(&mut self.ctx)?;
		match &self.resolution { Some(v) => v.inject(&mut self.ctx)?, None => (), }
		match &self.wm { Some(v) => v.inject(&mut self.ctx)?, None => (), }
		match &self.de { Some(v) => v.inject(&mut self.ctx)?, None => (), }
		match &self.cpu { Some(v) => v.inject(&mut self.ctx)?, None => (), }
		match &self.gpu { Some(v) => v.inject(&mut self.ctx)?, None => (), }
        self.render()?;
		{
			let plaintext = {
				let regex = Regex::new(r#"(?i)\[(?:[\d;]*\d+[a-z])"#).unwrap();
				String::from(regex.replace_all(&self.rendered, ""))
			};

			let mut w = 0usize;
			let mut h = 0usize;
			
			for line in plaintext.split("\n").collect::<Vec<&str>>() {
				{
					let len = line.chars().collect::<Vec<char>>().len();
					if len > w { w = len; }
				}
				h += 1;
			}

			self.width = w as i32;
			self.height = h as i32;
		}
		Ok(())
	}
	fn inject(&self, clml: &mut CLML) -> Result<(), ()> {
		// Inject clml values.
		clml
			.env("info", &format!("{}", self.rendered))
			.env("info.width", &format!("{}", self.width))
			.env("info.height", &format!("{}", self.height));

		// Inject bash values.
		clml
			.bash_env("info", &format!("{}", self.rendered))
			.env("info_width", &format!("{}", self.width))
			.env("info_height", &format!("{}", self.height));

		// Inject Lua values.
		{
			let lua = &clml.lua_env;
			let globals = lua.globals();

			match globals.set("info", self.rendered.as_str()) {
				Ok(_) => (),
				Err(e) => errors::handle(&format!("{}{}", errors::LUA, e)),
			}
			match globals.set("infoWidth", self.width) {
				Ok(_) => (),
				Err(e) => errors::handle(&format!("{}{}", errors::LUA, e)),
			}
			match globals.set("infoHeight", self.height) {
				Ok(_) => (),
				Err(e) => errors::handle(&format!("{}{}", errors::LUA, e)),
			}
		}

		Ok(())
	}
}
