use std::sync::Arc;

use async_trait::async_trait;

use crate::bf_declare;
use crate::compiler::builtins::offset_for_builtin;
use crate::model::r#match::{ArgSpec, PrepSpec, VerbArgsSpec};
use crate::model::verbs::VerbFlag;
use crate::util::bitenum::BitEnum;
use crate::values::error::Error::{E_INVARG, E_TYPE};
use crate::values::var::{v_err, v_list, v_none, v_objid, v_str, v_string, Var};
use crate::values::variant::Variant;
use crate::vm::builtin::{BfCallState, BuiltinFunction};
use crate::vm::VM;

// verb_info (obj <object>, str <verb-desc>) ->  {<owner>, <perms>, <names>}
async fn bf_verb_info<'a>(bf_args: &mut BfCallState<'a>) -> Result<Var, anyhow::Error> {
    if bf_args.args.len() != 2 {
        return Ok(v_err(E_INVARG));
    }
    let Variant::Obj(obj) = bf_args.args[0].variant() else {
        return Ok(v_err(E_TYPE));
    };

    let verb_info = match bf_args.args[1].variant() {
        Variant::Str(verb_desc) => {
            bf_args
                .world_state
                .get_verb(bf_args.perms(), *obj, verb_desc.as_str())?
        }
        Variant::Int(verb_index) => {
            let verb_index = *verb_index;
            if verb_index < 1 {
                return Ok(v_err(E_INVARG));
            }
            let verb_index = (verb_index as usize) - 1;
            bf_args
                .world_state
                .get_verb_at_index(bf_args.perms(), *obj, verb_index)?
        }
        _ => {
            return Ok(v_err(E_TYPE));
        }
    };
    let owner = verb_info.attrs.owner.unwrap();
    let perms = verb_info.attrs.flags.unwrap();
    let names = verb_info.names;

    let mut perms_string = String::new();
    if perms.contains(VerbFlag::Read) {
        perms_string.push('r');
    }
    if perms.contains(VerbFlag::Write) {
        perms_string.push('w');
    }
    if perms.contains(VerbFlag::Exec) {
        perms_string.push('x');
    }
    if perms.contains(VerbFlag::Debug) {
        perms_string.push('d');
    }

    // Join names into a single string, this is how MOO presents it.
    let verb_names = names.join(" ");

    let result = v_list(vec![
        v_objid(owner),
        v_string(perms_string),
        v_string(verb_names),
    ]);
    Ok(result)
}
bf_declare!(verb_info, bf_verb_info);

// set_verb_info (obj <object>, str <verb-desc>, list <info>) => none
async fn bf_set_verb_info<'a>(bf_args: &mut BfCallState<'a>) -> Result<Var, anyhow::Error> {
    if bf_args.args.len() != 3 {
        return Ok(v_err(E_INVARG));
    }
    let Variant::Obj(obj) = bf_args.args[0].variant() else {
        return Ok(v_err(E_TYPE));
    };
    let Variant::List(info) = bf_args.args[2].variant() else {
        return Ok(v_err(E_TYPE));
    };
    if info.len() != 3 {
        return Ok(v_err(E_INVARG));
    }
    match (info[0].variant(), info[1].variant(), info[2].variant()) {
        (Variant::Obj(owner), Variant::Str(perms_str), Variant::Str(names)) => {
            let mut perms = BitEnum::new();
            for c in perms_str.as_str().chars() {
                match c {
                    'r' => perms |= VerbFlag::Read,
                    'w' => perms |= VerbFlag::Write,
                    'x' => perms |= VerbFlag::Exec,
                    'd' => perms |= VerbFlag::Debug,
                    _ => return Ok(v_err(E_INVARG)),
                }
            }

            // Split the names string into a list of strings.
            let name_strings = names
                .as_str()
                .split(' ')
                .map(|s| s.into())
                .collect::<Vec<_>>();

            match bf_args.args[1].variant() {
                Variant::Str(verb_name) => {
                    bf_args.world_state.set_verb_info(
                        bf_args.perms(),
                        *obj,
                        verb_name.as_str(),
                        Some(*owner),
                        Some(name_strings),
                        Some(perms),
                        None,
                    )?;
                }
                Variant::Int(verb_index) => {
                    let verb_index = *verb_index;
                    if verb_index < 1 {
                        return Ok(v_err(E_INVARG));
                    }
                    let verb_index = (verb_index as usize) - 1;
                    bf_args.world_state.set_verb_info_at_index(
                        bf_args.perms(),
                        *obj,
                        verb_index,
                        Some(*owner),
                        Some(name_strings),
                        Some(perms),
                        None,
                    )?;
                }
                _ => return Ok(v_err(E_TYPE)),
            }

            Ok(v_none())
        }
        _ => Ok(v_err(E_INVARG)),
    }
}
bf_declare!(set_verb_info, bf_set_verb_info);

async fn bf_verb_args<'a>(bf_args: &mut BfCallState<'a>) -> Result<Var, anyhow::Error> {
    if bf_args.args.len() != 2 {
        return Ok(v_err(E_INVARG));
    }
    let Variant::Obj(obj) = bf_args.args[0].variant() else {
        return Ok(v_err(E_TYPE));
    };
    let args = match bf_args.args[1].variant() {
        Variant::Str(verb_desc) => {
            let verb_desc = verb_desc.as_str();
            let verb_info = bf_args
                .world_state
                .get_verb(bf_args.perms(), *obj, verb_desc)?;
            verb_info.attrs.args_spec.unwrap()
        }
        Variant::Int(verb_index) => {
            let verb_index = *verb_index;
            if verb_index < 1 {
                return Ok(v_err(E_INVARG));
            }
            let verb_index = (verb_index as usize) - 1;
            let verb_info =
                bf_args
                    .world_state
                    .get_verb_at_index(bf_args.perms(), *obj, verb_index)?;
            verb_info.attrs.args_spec.unwrap()
        }
        _ => return Ok(v_err(E_TYPE)),
    };
    // Output is {dobj, prep, iobj} as strings
    let result = v_list(vec![
        v_str(args.dobj.to_string()),
        v_str(args.prep.to_string()),
        v_str(args.iobj.to_string()),
    ]);
    Ok(result)
}
bf_declare!(verb_args, bf_verb_args);

// set_verb_args (obj <object>, str <verb-desc>, list <args>) => none
async fn bf_set_verb_args<'a>(bf_args: &mut BfCallState<'a>) -> Result<Var, anyhow::Error> {
    if bf_args.args.len() != 3 {
        return Ok(v_err(E_INVARG));
    }
    let Variant::Obj(obj) = bf_args.args[0].variant() else {
        return Ok(v_err(E_TYPE));
    };
    let Variant::List(verbinfo) = bf_args.args[2].variant() else {
        return Ok(v_err(E_TYPE));
    };
    if verbinfo.len() != 3 {
        return Ok(v_err(E_INVARG));
    }
    match (
        verbinfo[0].variant(),
        verbinfo[1].variant(),
        verbinfo[2].variant(),
    ) {
        (Variant::Str(dobj_str), Variant::Str(prep_str), Variant::Str(iobj_str)) => {
            let Some(dobj) = ArgSpec::from_string(dobj_str.as_str()) else {
                return Ok(v_err(E_INVARG));
            };
            let Some(prep) = PrepSpec::from_string(prep_str.as_str()) else {
                return Ok(v_err(E_INVARG));
            };
            let Some(iobj) = ArgSpec::from_string(iobj_str.as_str()) else {
                return Ok(v_err(E_INVARG));
            };
            let args = VerbArgsSpec { dobj, prep, iobj };
            match bf_args.args[1].variant() {
                Variant::Str(verb_name) => {
                    bf_args.world_state.set_verb_info(
                        bf_args.perms(),
                        *obj,
                        verb_name.as_str(),
                        None,
                        None,
                        None,
                        Some(args),
                    )?;
                }
                Variant::Int(verb_index) => {
                    let verb_index = *verb_index;
                    if verb_index < 1 {
                        return Ok(v_err(E_INVARG));
                    }
                    let verb_index = (verb_index as usize) - 1;
                    bf_args.world_state.set_verb_info_at_index(
                        bf_args.perms(),
                        *obj,
                        verb_index,
                        None,
                        None,
                        None,
                        Some(args),
                    )?;
                }
                _ => return Ok(v_err(E_TYPE)),
            }
            Ok(v_none())
        }
        _ => Ok(v_err(E_INVARG)),
    }
}
bf_declare!(set_verb_args, bf_set_verb_args);

impl VM {
    pub(crate) fn register_bf_verbs(&mut self) -> Result<(), anyhow::Error> {
        self.builtins[offset_for_builtin("verb_info")] = Arc::new(Box::new(BfVerbInfo {}));
        self.builtins[offset_for_builtin("set_verb_info")] = Arc::new(Box::new(BfSetVerbInfo {}));
        self.builtins[offset_for_builtin("verb_args")] = Arc::new(Box::new(BfVerbArgs {}));
        self.builtins[offset_for_builtin("set_verb_args")] = Arc::new(Box::new(BfSetVerbArgs {}));

        Ok(())
    }
}
