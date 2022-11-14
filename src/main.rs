use crate::grammar::moolexer::mooLexer;
use antlr_rust::{InputStream, Parser};


use std::fs::{File};

use std::io::BufReader;

use antlr_rust::common_token_stream::CommonTokenStream;





use antlr_rust::tree::{ParseTree};

use symbol_table::{SymbolTable};
use crate::compiler::parse::VerbCompileErrorListener;


use crate::grammar::mooparser::{mooParser};
use crate::textdump::{TextdumpReader};

pub mod grammar;
pub mod model;
pub mod textdump;
pub mod compiler;

fn main() {
    println!("Hello, world!");

    let jhcore = File::open("JHCore-DEV-2.db").unwrap();

    let br = BufReader::new(jhcore);

    let _symtab = SymbolTable::new();
    let mut tdr = TextdumpReader::new(br);

    let td=  tdr.read_textdump().unwrap();

    // Now iterate and compile each verb...
    for v in &td.verbs {
        println!("Compiling verb {}:{}", v.objid.0, v.verbnum);
        let is = InputStream::new(v.program.as_str());
        let lexer = mooLexer::new(is);
        let source = CommonTokenStream::new(lexer);
        let mut parser = mooParser::new(source);
        println!("Compiled");
        
        let err_listener = Box::new(VerbCompileErrorListener { program: v.program.clone() });
        
        parser.add_error_listener(err_listener);
        let program_context = parser.program().unwrap();

        let _tree = program_context.to_string_tree(&*parser);
    }
}
