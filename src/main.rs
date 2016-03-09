extern crate hyper;
extern crate gtk;
extern crate gdk;

use std::io::Read;
use std::rc::Rc;
use std::cell::RefCell;

use hyper::Client;
use hyper::header::Connection;

use gtk::traits::*;
use gtk::{
    Window,
    Builder,
    Button,
    ButtonSignals,
    ComboBoxText,
    Inhibit,
    TextView,
    TextBuffer,
    TextTagTable,
    WidgetSignals
};
use gdk::enums::key;

const TRANSLATE: &'static str = "http://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=";

fn main() {
    // Initialize GTK
    if let Err(message) = gtk::init() {
        panic!("{:?}", message);
    }

    // Open the UI that we created in Glade
    let glade_src = include_str!("translate.glade");
    let builder = Builder::new_from_string(glade_src);

    // Grab the elements from the UI
    let window: Window = builder.get_object("main_window").unwrap();
    let translate_button: Button = builder.get_object("translate_button").unwrap();
    let translation_input: TextView = builder.get_object("translation_input").unwrap();
    let translation_output: TextView = builder.get_object("translation_output").unwrap();
    let language_box: ComboBoxText = builder.get_object("language").unwrap();

    // Add a TextBuffer to every TextView
    let input_buffer = TextBuffer::new(Some(&TextTagTable::new()));
    translation_input.set_buffer(Some(&input_buffer));
    let output_buffer = TextBuffer::new(Some(&TextTagTable::new()));
    translation_output.set_buffer(Some(&output_buffer));

    // Wrap translation_button so that it may be borrowed multiple times
    let wrapped_translation_button = Rc::new(RefCell::new(translate_button));

    {   // Take the input buffer, translate it, and output it to the outbut buffer.
        let translate_button = wrapped_translation_button.clone();
        translate_button.borrow().connect_clicked(move |_| {
            // Get the input buffer's text
            let buffer = translation_input.get_buffer().unwrap();
            let string = buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false).unwrap();

            // Get the langauge combo box's text.
            let language = language_box.get_active_text().unwrap();

            // Translate the text.
            let mut translation = String::new();
            translate(&string, language.as_str(), &mut translation);

            // Update the translation output text view.
            translation_output.get_buffer().unwrap().set_text(translation.as_str());
        });
    }

    // Exit the program if it receives the delete event.
    window.connect_delete_event(|_,_| {
        gtk::main_quit();
        Inhibit(false)
    });

    { // Program what the program should do when certain keys are pressed
        let translate_button = wrapped_translation_button.clone();
        window.connect_key_press_event(move |_,key| {
            match key.get_keyval() {
                key::Escape => gtk::main_quit(),
                key::Return  => translate_button.borrow().clicked(),
                _ => ()
            }
            Inhibit(false)
        });
    }

    // Show the window and start the program
    window.show_all();
    gtk::main();
}

/// Send text to Google Translate and translate it.
fn translate(input: &str, language: &str, output: &mut String) {
    let mut search = String::new();
    search.push_str(TRANSLATE);
    search.push_str(language);
    search.push_str("&dt=t&q=");
    search.push_str(input);
    if let Ok(mut response) = Client::new().get(&search).header(Connection::close()).send() {
        search.clear();
        if let Err(error) = response.read_to_string(&mut search) {
            panic!("Unable to read response: {}", error);
        }
    }
    parse_message(search.as_str(), output);
}

/// Take the raw response from Google and parse the translation only.
fn parse_message(input: &str, translation: &mut String) {
    let mut iterator    = input.chars().skip(4);
    let mut escape      = false;
    let mut ignore      = false;
    let mut found_match = false;
    let mut matched: i8 = 0;

    // Loop until ',,,0]]' is found
    while let Some(character) = iterator.next() {
        if found_match {
            matched = match matched {
                0 => 1,
                1 => { found_match = false; 0 },
                _     => unreachable!()
            }
        } else if ignore {
            matched = match matched {
                0 => if character == ',' { 1 } else { 0 },
                1 => if character == ',' { 2 } else { 0 },
                2 => if character == ',' { 3 } else { 0 },
                3 => if character == '0' { 4 } else { 0 },
                4 => if character == ']' { 5 } else { 0 },
                5 => match character {
                    ']' => break, // ',,,0]]' has been found
                    _   => {ignore = false; found_match = true; 0 }
                },
                _ => 0
            };
        } else {
            if character == '\\' && !escape {
                escape = true;
            } else if escape {
                translation.push(character);
                escape = false;
            } else if character == '"' {
                ignore = true;
            } else {
                translation.push(character);
            }
        }
    }
}


#[test]
fn test_parse_message() {
    const TEST: &'static str = "[[[\"I am not you. \",\"Mi estas ne vin.\",,,0],[\"You are not me.\",\"Vi estas ne min.\",,,0]],,\"eo\",,,,0.070792444,,[[\"eo\"],,[0.070792444],[\"eo\"]]]";
    let mut output = String::new();
    parse_message(TEST, &mut output);
    assert_eq!(output.as_str(), "I am not you. You are not me.")
}
