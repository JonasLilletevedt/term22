//  My own imports
mod term22;

use term22::Term22;

use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::ffi::CString;
use nix::pty::{forkpty, ForkptyResult, PtyMaster};
use nix::unistd::{self, Pid};

// Egui
use eframe::egui;



fn main() {
    env_logger::init();

    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "term22", 
        options, 
        Box::new(|_cc| Box::new(Term22::default())),);


    

    
    // TODO
    // Clear method
    // Cursor
    // Colours, fonts, size ...
    // Convert raw output to gui
    // Custom configs:
    // - Aliases, fonts, colourstyles ... 
    //
    // Extra
    // Simple terminal game

}


