use std::ffi::CString;
use std::os::unix::io::{OwnedFd, AsRawFd, FromRawFd, IntoRawFd};
use std::u8;
use eframe::App;
use egui::{output, viewport, CentralPanel};
use nix::pty::{forkpty, ForkptyResult};
use nix::unistd::Pid;
use nix::Error as NixError;


pub struct Term22 {
    current_line_input: String,
    displayed_output: String,

    shell_path: String,
    master_fd: OwnedFd,
    child_shell_pid: Pid,
}

impl Term22 {
    fn handle_key_events(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Close app on esc
            ui.input(|input| {
                for event in &input.events {
                    match event {
                        egui::Event::Text(text) => handle_text_input(self, text),
                        egui::Event::Key { key, physical_key, pressed, repeat, modifiers } => handle_special_keys(self, *key, ctx),
                        _ => ()
                    }
                }
            })
        });

        fn handle_special_keys(app_state: &mut Term22, key_event: egui::Key, ctx: &egui::Context) {
            match key_event {
                egui::Key::Enter => {
                    if app_state.current_line_input.is_empty() { return; }
                    let command_to_write = app_state.current_line_input.clone() + "\n";
                    app_state.displayed_output.push_str(&command_to_write);
                    let buffer_to_write = command_to_write.as_bytes();
                    let _ = nix::unistd::write(&app_state.master_fd, buffer_to_write);
                    app_state.current_line_input.clear();

                    read_from_master_fd(app_state);
                },
                egui::Key::Backspace => {
                    app_state.current_line_input.pop();
                },
                egui::Key::Escape => {
                    println!("[Term22::App::update] Attempting to close application.");
                    // TODO
                    // is a workaround for an apparent freeze/unresponsiveness issue when calling
                    // it directly from App::update under certain conditions.
                    let ctx2 = ctx.clone();
                    std::thread::spawn(move || {
                        ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
                    });
                },
                _ => ()
            }
        }
        fn handle_text_input(app_state: &mut Term22, text: &str) {
            app_state.current_line_input.push_str(text);
        }

        fn read_from_master_fd(app_state: &mut Term22) {
            let mut output_buffer: [u8; 4096] = [0; 4096];
            match nix::unistd::read(app_state.master_fd.as_raw_fd(), &mut output_buffer) {
                Ok(n_bytes) => {
                    let output_str = String::from_utf8_lossy(&output_buffer[..n_bytes]) + "\n";
                    app_state.displayed_output.push_str(&output_str);
                    println!("[PTY Output]: {}", output_str); 
                }
                Err(e) => {
                    let error_msg = format!("\n[Error reading from PTY: {}]\n", e);
                    app_state.displayed_output.push_str(&error_msg);
                    eprintln!("{}", error_msg);
                },
            }

        }

    }
}

impl Default for Term22 {
    fn default() -> Self {
        let shell_path_str = get_default_shell_path();
        println!("[Term22::default] Attempting to initalize PTY with  shell:  {}", shell_path_str);
        let (fork_master_fd, fork_child_pid) = match init_pty(&shell_path_str) {
            Ok((the_fd, the_pid)) => {
                println!("[Term22::default] PTY initalized succesfully. Master Fd {},Child Pid: {}", 
                    the_fd.as_raw_fd(), the_pid);
                (the_fd, the_pid)
            }
            Err(e) => {
                eprintln!("[Term22::default] Failed to initalize PTY: {}", e);
                panic!("Application (Term22) cannot start without a PTY.");
            }
        };

        Self {
            current_line_input: String::new(),

            displayed_output: format!("Welcome! Shell: {}\nPTY Master FD: {}, Child PID: {}\n",
                shell_path_str,
                fork_master_fd.as_raw_fd(),
                fork_child_pid),

                shell_path: shell_path_str,
                master_fd: fork_master_fd, 
                child_shell_pid: fork_child_pid,

        }
    }
}



impl App for Term22 {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Term22::handle_key_events(self, ctx);
            let mut output = self.displayed_output.clone();
            output.push_str(&self.current_line_input);
            ui.label(output);
        });
    }
}

fn get_default_shell_path() -> String {
    std::env::var("SHELL")
        .expect("could not find default shell from $SHELL")
}



fn init_pty(shell_path_str: &str) -> Result<(OwnedFd, Pid), NixError> {
    unsafe {
        match forkpty(None, None) {
            Ok(fork_result) => {
                match fork_result {
                    ForkptyResult::Parent { master, child } => {
                        let raw_fd = master.into_raw_fd();
                        Ok((OwnedFd::from_raw_fd(raw_fd), child))
                    }
                    ForkptyResult::Child => {
                        let shell_cstr = CString::new(shell_path_str)
                            .expect("Child: Failed to create CString for shell path");
                        let arg = [shell_cstr.as_c_str()];
                        match nix::unistd::execvp(&shell_cstr, &arg) {
                            Ok(_) => unreachable!(),
                            Err(e) => {
                                eprintln!("[Child] FATAL: execvp failed for '{}': {}", shell_path_str, e);
                                std::process::exit(127);
                            }
                        }
                    }
                }
            },      
            Err(e) => {
                eprintln!("Err: forkpty failed in init_pty: {}", e);
                Err(e)
            }
        }
    }
}

