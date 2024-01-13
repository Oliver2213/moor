// Copyright (C) 2024 Ryan Daum <ryan.daum@gmail.com>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

use async_trait::async_trait;
use uuid::Uuid;

use moor_values::model::defset::HasUuid;
use moor_values::model::objects::{ObjAttrs, ObjFlag};
use moor_values::model::objset::ObjSet;
use moor_values::model::permissions::Perms;
use moor_values::model::propdef::{PropDef, PropDefs};
use moor_values::model::props::{PropAttrs, PropFlag};
use moor_values::model::r#match::{ArgSpec, PrepSpec, VerbArgsSpec};
use moor_values::model::verb_info::VerbInfo;
use moor_values::model::verbdef::{VerbDef, VerbDefs};
use moor_values::model::verbs::{BinaryType, VerbAttrs, VerbFlag};
use moor_values::model::world_state::WorldState;
use moor_values::model::CommitResult;
use moor_values::model::WorldStateError;
use moor_values::util::bitenum::BitEnum;
use moor_values::util::slice_ref::SliceRef;
use moor_values::var::objid::Objid;
use moor_values::var::variant::Variant;
use moor_values::var::{v_int, v_list, v_objid, Var};
use moor_values::NOTHING;

use crate::db_tx::DbTransaction;

pub struct DbTxWorldState {
    pub tx: Box<dyn DbTransaction + Send + Sync>,
}

impl DbTxWorldState {
    async fn perms(&self, who: Objid) -> Result<Perms, WorldStateError> {
        let flags = self.flags_of(who).await?;
        Ok(Perms { who, flags })
    }
}

#[async_trait]
impl WorldState for DbTxWorldState {
    async fn players(&self) -> Result<ObjSet, WorldStateError> {
        self.tx.get_players().await
    }

    #[tracing::instrument(skip(self))]
    async fn owner_of(&self, obj: Objid) -> Result<Objid, WorldStateError> {
        self.tx.get_object_owner(obj).await
    }

    #[tracing::instrument(skip(self))]
    async fn flags_of(&self, obj: Objid) -> Result<BitEnum<ObjFlag>, WorldStateError> {
        self.tx.get_object_flags(obj).await
    }

    async fn set_flags_of(
        &mut self,
        perms: Objid,
        obj: Objid,
        new_flags: BitEnum<ObjFlag>,
    ) -> Result<(), WorldStateError> {
        // Owner or wizard only.
        let (flags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, flags, ObjFlag::Write)?;
        self.tx.set_object_flags(obj, new_flags).await
    }

    #[tracing::instrument(skip(self))]
    async fn location_of(&self, _perms: Objid, obj: Objid) -> Result<Objid, WorldStateError> {
        // MOO permits location query even if the object is unreadable!
        self.tx.get_object_location(obj).await
    }

    async fn object_bytes(&self, perms: Objid, obj: Objid) -> Result<usize, WorldStateError> {
        self.perms(perms).await?.check_wizard()?;
        self.tx.get_object_size_bytes(obj).await
    }

    #[tracing::instrument(skip(self))]
    async fn create_object(
        &mut self,
        perms: Objid,
        parent: Objid,
        owner: Objid,
        flags: BitEnum<ObjFlag>,
    ) -> Result<Objid, WorldStateError> {
        if parent != NOTHING {
            let (flags, parent_owner) =
                (self.flags_of(parent).await?, self.owner_of(parent).await?);
            // TODO check_object_allows should take a BitEnum arg for `allows` and do both of these at
            // once.
            self.perms(perms)
                .await?
                .check_object_allows(parent_owner, flags, ObjFlag::Read)?;
            self.perms(perms)
                .await?
                .check_object_allows(parent_owner, flags, ObjFlag::Fertile)?;
        }

        let owner = (owner != NOTHING).then_some(owner);

        /*
            TODO: quota:
            If the intended owner of the new object has a property named `ownership_quota' and the value of that property is an integer, then `create()' treats that value
            as a "quota".  If the quota is less than or equal to zero, then the quota is considered to be exhausted and `create()' raises `E_QUOTA' instead of creating an
            object.  Otherwise, the quota is decremented and stored back into the `ownership_quota' property as a part of the creation of the new object.
        */
        let attrs = ObjAttrs {
            owner,
            name: None,
            parent: Some(parent),
            location: None,
            flags: Some(flags),
        };
        self.tx.create_object(None, attrs).await
    }

    async fn recycle_object(&mut self, perms: Objid, obj: Objid) -> Result<(), WorldStateError> {
        let (flags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, flags, ObjFlag::Write)?;

        self.tx.recycle_object(obj).await
    }

    async fn max_object(&self, _perms: Objid) -> Result<Objid, WorldStateError> {
        self.tx.get_max_object().await
    }

    async fn move_object(
        &mut self,
        perms: Objid,
        obj: Objid,
        new_loc: Objid,
    ) -> Result<(), WorldStateError> {
        let (flags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, flags, ObjFlag::Write)?;

        self.tx.set_object_location(obj, new_loc).await
    }

    #[tracing::instrument(skip(self))]
    async fn contents_of(&self, _perms: Objid, obj: Objid) -> Result<ObjSet, WorldStateError> {
        // MOO does not do any perms checks on contents, pretty sure:
        // https://github.com/wrog/lambdamoo/blob/master/db_properties.c#L351
        self.tx.get_object_contents(obj).await
    }

    #[tracing::instrument(skip(self))]
    async fn verbs(&self, perms: Objid, obj: Objid) -> Result<VerbDefs, WorldStateError> {
        let (flags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, flags, ObjFlag::Read)?;

        Ok(self.tx.get_verbs(obj).await?)
    }

    #[tracing::instrument(skip(self))]
    async fn properties(&self, perms: Objid, obj: Objid) -> Result<PropDefs, WorldStateError> {
        let (flags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, flags, ObjFlag::Read)?;

        let properties = self.tx.get_properties(obj).await?;
        Ok(properties)
    }

    #[tracing::instrument(skip(self))]
    async fn retrieve_property(
        &self,
        perms: Objid,
        obj: Objid,
        pname: &str,
    ) -> Result<Var, WorldStateError> {
        if obj == NOTHING || !self.valid(obj).await? {
            return Err(WorldStateError::ObjectNotFound(obj));
        }

        // Special properties like namnne, location, and contents get treated specially.
        if pname == "name" {
            return self
                .names_of(perms, obj)
                .await
                .map(|(name, _)| Var::from(name));
        } else if pname == "location" {
            return self.location_of(perms, obj).await.map(Var::from);
        } else if pname == "contents" {
            let contents: Vec<_> = self
                .contents_of(perms, obj)
                .await?
                .iter()
                .map(v_objid)
                .collect();
            return Ok(v_list(&contents));
        } else if pname == "owner" {
            return self.owner_of(obj).await.map(Var::from);
        } else if pname == "programmer" {
            // TODO these can be set, too.
            let flags = self.flags_of(obj).await?;
            return if flags.contains(ObjFlag::Programmer) {
                Ok(v_int(1))
            } else {
                Ok(v_int(0))
            };
        } else if pname == "wizard" {
            let flags = self.flags_of(obj).await?;
            return if flags.contains(ObjFlag::Wizard) {
                Ok(v_int(1))
            } else {
                Ok(v_int(0))
            };
        } else if pname == "r" {
            let flags = self.flags_of(obj).await?;
            return if flags.contains(ObjFlag::Read) {
                Ok(v_int(1))
            } else {
                Ok(v_int(0))
            };
        } else if pname == "w" {
            let flags = self.flags_of(obj).await?;
            return if flags.contains(ObjFlag::Write) {
                Ok(v_int(1))
            } else {
                Ok(v_int(0))
            };
        } else if pname == "f" {
            let flags = self.flags_of(obj).await?;
            return if flags.contains(ObjFlag::Fertile) {
                Ok(v_int(1))
            } else {
                Ok(v_int(0))
            };
        }

        let (ph, value) = self.tx.resolve_property(obj, pname.to_string()).await?;
        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Read)?;
        Ok(value)
    }

    async fn get_property_info(
        &self,
        perms: Objid,
        obj: Objid,
        pname: &str,
    ) -> Result<PropDef, WorldStateError> {
        let properties = self.tx.get_properties(obj).await?;
        let ph = properties
            .find_first_named(pname)
            .ok_or(WorldStateError::PropertyNotFound(obj, pname.into()))?;
        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Read)?;

        Ok(ph.clone())
    }

    async fn set_property_info(
        &mut self,
        perms: Objid,
        obj: Objid,
        pname: &str,
        attrs: PropAttrs,
    ) -> Result<(), WorldStateError> {
        let properties = self.tx.get_properties(obj).await?;
        let ph = properties
            .find_first_named(pname)
            .ok_or(WorldStateError::PropertyNotFound(obj, pname.into()))?;

        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Write)?;

        // TODO Also keep a close eye on 'clear' & perms:
        //  "raises `E_INVARG' if <owner> is not valid" & If <object> is the definer of the property
        //   <prop-name>, as opposed to an inheritor of the property, then `clear_property()' raises
        //   `E_INVARG'

        self.tx
            .update_property_definition(obj, ph.uuid(), attrs.owner, attrs.flags, attrs.name)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update_property(
        &mut self,
        perms: Objid,
        obj: Objid,
        pname: &str,
        value: &Var,
    ) -> Result<(), WorldStateError> {
        // You have to use move/chparent for this kinda fun.
        if pname == "location" || pname == "contents" || pname == "parent" || pname == "children" {
            return Err(WorldStateError::PropertyPermissionDenied);
        }

        if pname == "name" || pname == "owner" || pname == "r" || pname == "w" || pname == "f" {
            let (mut flags, objowner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);

            // User is either wizard or owner
            self.perms(perms)
                .await?
                .check_object_allows(objowner, flags, ObjFlag::Write)?;
            if pname == "name" {
                let Variant::Str(name) = value.variant() else {
                    return Err(WorldStateError::PropertyTypeMismatch);
                };
                self.tx.set_object_name(obj, name.to_string()).await?;
                return Ok(());
            }

            if pname == "owner" {
                let Variant::Obj(owner) = value.variant() else {
                    return Err(WorldStateError::PropertyTypeMismatch);
                };
                self.tx.set_object_owner(obj, *owner).await?;
                return Ok(());
            }

            if pname == "r" {
                let Variant::Int(v) = value.variant() else {
                    return Err(WorldStateError::PropertyTypeMismatch);
                };
                if *v == 1 {
                    flags.set(ObjFlag::Read);
                } else {
                    flags.clear(ObjFlag::Read);
                }
                self.tx.set_object_flags(obj, flags).await?;
                return Ok(());
            }

            if pname == "w" {
                let Variant::Int(v) = value.variant() else {
                    return Err(WorldStateError::PropertyTypeMismatch);
                };
                if *v == 1 {
                    flags.set(ObjFlag::Write);
                } else {
                    flags.clear(ObjFlag::Write);
                }
                self.tx.set_object_flags(obj, flags).await?;
                return Ok(());
            }

            if pname == "f" {
                let Variant::Int(v) = value.variant() else {
                    return Err(WorldStateError::PropertyTypeMismatch);
                };
                if *v == 1 {
                    flags.set(ObjFlag::Fertile);
                } else {
                    flags.clear(ObjFlag::Fertile);
                }
                self.tx.set_object_flags(obj, flags).await?;
                return Ok(());
            }
        }

        if pname == "programmer" || pname == "wizard" {
            // Caller *must* be a wizard for either of these.
            self.perms(perms).await?.check_wizard()?;

            // Gott get and then set flags
            let mut flags = self.flags_of(obj).await?;
            if pname == "programmer" {
                flags.set(ObjFlag::Programmer);
            } else if pname == "wizard" {
                flags.set(ObjFlag::Wizard);
            }

            self.tx.set_object_flags(obj, flags).await?;
            return Ok(());
        }

        let properties = self.tx.get_properties(obj).await?;
        let ph = properties
            .find_first_named(pname)
            .ok_or(WorldStateError::PropertyNotFound(obj, pname.into()))?;

        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Write)?;

        self.tx.set_property(obj, ph.uuid(), value.clone()).await?;
        Ok(())
    }

    async fn is_property_clear(
        &self,
        perms: Objid,
        obj: Objid,
        pname: &str,
    ) -> Result<bool, WorldStateError> {
        let properties = self.tx.get_properties(obj).await?;
        let ph = properties
            .find_first_named(pname)
            .ok_or(WorldStateError::PropertyNotFound(obj, pname.into()))?;
        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Read)?;

        // Now RetrieveProperty and if it's not there, it's clear.
        let result = self.tx.retrieve_property(obj, ph.uuid()).await;
        // What we want is an ObjectError::PropertyNotFound, that will tell us if it's clear.
        let is_clear = match result {
            Err(WorldStateError::PropertyNotFound(_, _)) => true,
            Ok(_) => false,
            Err(e) => return Err(e),
        };
        Ok(is_clear)
    }

    async fn clear_property(
        &mut self,
        perms: Objid,
        obj: Objid,
        pname: &str,
    ) -> Result<(), WorldStateError> {
        // This is just deleting the local *value* portion of the property.
        // First seek the property handle.
        let properties = self.tx.get_properties(obj).await?;
        let ph = properties
            .find_first_named(pname)
            .ok_or(WorldStateError::PropertyNotFound(obj, pname.into()))?;
        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Write)?;

        self.tx.clear_property(obj, ph.uuid()).await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn define_property(
        &mut self,
        perms: Objid,
        definer: Objid,
        location: Objid,
        pname: &str,
        propowner: Objid,
        prop_flags: BitEnum<PropFlag>,
        initial_value: Option<Var>,
    ) -> Result<(), WorldStateError> {
        // Perms needs to be wizard, or have write permission on object *and* the owner in prop_flags
        // must be the perms
        let (flags, objowner) = (
            self.flags_of(location).await?,
            self.owner_of(location).await?,
        );
        self.perms(perms)
            .await?
            .check_object_allows(objowner, flags, ObjFlag::Write)?;
        self.perms(perms).await?.check_obj_owner_perms(propowner)?;

        self.tx
            .define_property(
                definer,
                location,
                pname.to_string(),
                propowner,
                prop_flags,
                initial_value,
            )
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete_property(
        &mut self,
        perms: Objid,
        obj: Objid,
        pname: &str,
    ) -> Result<(), WorldStateError> {
        let properties = self.tx.get_properties(obj).await?;
        let ph = properties
            .find_first_named(pname)
            .ok_or(WorldStateError::PropertyNotFound(obj, pname.into()))?;
        self.perms(perms)
            .await?
            .check_property_allows(ph.owner(), ph.flags(), PropFlag::Write)?;

        self.tx.delete_property(obj, ph.uuid()).await
    }

    #[tracing::instrument(skip(self))]
    async fn add_verb(
        &mut self,
        perms: Objid,
        obj: Objid,
        names: Vec<String>,
        owner: Objid,
        flags: BitEnum<VerbFlag>,
        args: VerbArgsSpec,
        binary: Vec<u8>,
        binary_type: BinaryType,
    ) -> Result<(), WorldStateError> {
        let (objflags, obj_owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(obj_owner, objflags, ObjFlag::Write)?;

        self.tx
            .add_object_verb(obj, owner, names, binary, binary_type, flags, args)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn remove_verb(
        &mut self,
        perms: Objid,
        obj: Objid,
        uuid: Uuid,
    ) -> Result<(), WorldStateError> {
        let verbs = self.tx.get_verbs(obj).await?;
        let vh = verbs
            .find(&uuid)
            .ok_or(WorldStateError::VerbNotFound(obj, uuid.to_string()))?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Write)?;

        self.tx.delete_verb(obj, vh.uuid()).await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update_verb(
        &mut self,
        perms: Objid,
        obj: Objid,
        vname: &str,
        verb_attrs: VerbAttrs,
    ) -> Result<(), WorldStateError> {
        let vh = self.tx.get_verb_by_name(obj, vname.to_string()).await?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Write)?;
        self.tx.update_verb(obj, vh.uuid(), verb_attrs).await?;
        Ok(())
    }

    async fn update_verb_at_index(
        &mut self,
        perms: Objid,
        obj: Objid,
        vidx: usize,
        verb_attrs: VerbAttrs,
    ) -> Result<(), WorldStateError> {
        let vh = self.tx.get_verb_by_index(obj, vidx).await?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Write)?;
        self.tx.update_verb(obj, vh.uuid(), verb_attrs).await?;
        Ok(())
    }

    async fn update_verb_with_id(
        &mut self,
        perms: Objid,
        obj: Objid,
        uuid: Uuid,
        verb_attrs: VerbAttrs,
    ) -> Result<(), WorldStateError> {
        let verbs = self.tx.get_verbs(obj).await?;
        let vh = verbs
            .find(&uuid)
            .ok_or(WorldStateError::VerbNotFound(obj, uuid.to_string()))?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Write)?;
        self.tx.update_verb(obj, vh.uuid(), verb_attrs).await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn get_verb(
        &self,
        perms: Objid,
        obj: Objid,
        vname: &str,
    ) -> Result<VerbDef, WorldStateError> {
        if !self.tx.object_valid(obj).await? {
            return Err(WorldStateError::ObjectNotFound(obj));
        }

        let vh = self.tx.get_verb_by_name(obj, vname.to_string()).await?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Read)?;

        Ok(vh)
    }

    async fn get_verb_at_index(
        &self,
        perms: Objid,
        obj: Objid,
        vidx: usize,
    ) -> Result<VerbDef, WorldStateError> {
        let vh = self.tx.get_verb_by_index(obj, vidx).await?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Read)?;
        Ok(vh)
    }

    async fn retrieve_verb(
        &self,
        perms: Objid,
        obj: Objid,
        uuid: Uuid,
    ) -> Result<VerbInfo, WorldStateError> {
        let verbs = self.tx.get_verbs(obj).await?;
        let vh = verbs
            .find(&uuid)
            .ok_or(WorldStateError::VerbNotFound(obj, uuid.to_string()))?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Read)?;
        let binary = self.tx.get_verb_binary(vh.location(), vh.uuid()).await?;
        Ok(VerbInfo::new(vh, SliceRef::from_vec(binary)))
    }

    #[tracing::instrument(skip(self))]
    async fn find_method_verb_on(
        &self,
        perms: Objid,
        obj: Objid,
        vname: &str,
    ) -> Result<VerbInfo, WorldStateError> {
        let vh = self.tx.resolve_verb(obj, vname.to_string(), None).await?;
        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Read)?;

        let binary = self.tx.get_verb_binary(vh.location(), vh.uuid()).await?;
        Ok(VerbInfo::new(vh, SliceRef::from_vec(binary)))
    }

    #[tracing::instrument(skip(self))]
    async fn find_command_verb_on(
        &self,
        perms: Objid,
        obj: Objid,
        command_verb: &str,
        dobj: Objid,
        prep: PrepSpec,
        iobj: Objid,
    ) -> Result<Option<VerbInfo>, WorldStateError> {
        if !self.valid(obj).await? {
            return Ok(None);
        }

        let (objflags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, objflags, ObjFlag::Read)?;

        let spec_for_fn = |oid, pco| -> ArgSpec {
            if pco == oid {
                ArgSpec::This
            } else if pco == NOTHING {
                ArgSpec::None
            } else {
                ArgSpec::Any
            }
        };

        let dobj = spec_for_fn(obj, dobj);
        let iobj = spec_for_fn(obj, iobj);
        let argspec = VerbArgsSpec { dobj, prep, iobj };

        let vh = self
            .tx
            .resolve_verb(obj, command_verb.to_string(), Some(argspec))
            .await;
        let vh = match vh {
            Ok(vh) => vh,
            Err(WorldStateError::VerbNotFound(_, _)) => {
                return Ok(None);
            }
            Err(e) => {
                return Err(e);
            }
        };

        self.perms(perms)
            .await?
            .check_verb_allows(vh.owner(), vh.flags(), VerbFlag::Read)?;

        let binary = self.tx.get_verb_binary(vh.location(), vh.uuid()).await?;
        Ok(Some(VerbInfo::new(vh, SliceRef::from_vec(binary))))
    }

    #[tracing::instrument(skip(self))]
    async fn parent_of(&self, _perms: Objid, obj: Objid) -> Result<Objid, WorldStateError> {
        self.tx.get_object_parent(obj).await
    }

    async fn change_parent(
        &mut self,
        perms: Objid,
        obj: Objid,
        new_parent: Objid,
    ) -> Result<(), WorldStateError> {
        if obj == new_parent {
            return Err(WorldStateError::RecursiveMove(obj, new_parent));
        }

        let (objflags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);

        if new_parent != NOTHING {
            let (parentflags, parentowner) = (
                self.flags_of(new_parent).await?,
                self.owner_of(new_parent).await?,
            );
            self.perms(perms).await?.check_object_allows(
                parentowner,
                parentflags,
                ObjFlag::Write,
            )?;
            self.perms(perms).await?.check_object_allows(
                parentowner,
                parentflags,
                ObjFlag::Fertile,
            )?;
        }
        self.perms(perms)
            .await?
            .check_object_allows(owner, objflags, ObjFlag::Write)?;

        self.tx.set_object_parent(obj, new_parent).await
    }

    #[tracing::instrument(skip(self))]
    async fn children_of(&self, perms: Objid, obj: Objid) -> Result<ObjSet, WorldStateError> {
        let (objflags, owner) = (self.flags_of(obj).await?, self.owner_of(obj).await?);
        self.perms(perms)
            .await?
            .check_object_allows(owner, objflags, ObjFlag::Read)?;

        self.tx.get_object_children(obj).await
    }

    #[tracing::instrument(skip(self))]
    async fn valid(&self, obj: Objid) -> Result<bool, WorldStateError> {
        self.tx.object_valid(obj).await
    }

    #[tracing::instrument(skip(self))]
    async fn names_of(
        &self,
        perms: Objid,
        obj: Objid,
    ) -> Result<(String, Vec<String>), WorldStateError> {
        // Another thing that MOO allows lookup of without permissions.
        // First get name
        let name = self.tx.get_object_name(obj).await?;

        // Then grab aliases property.
        let aliases = match self.retrieve_property(perms, obj, "aliases").await {
            Ok(a) => match a.variant() {
                Variant::List(a) => a.iter().map(|v| v.to_string()).collect(),
                _ => {
                    vec![]
                }
            },
            Err(_) => {
                vec![]
            }
        };

        Ok((name, aliases))
    }

    #[tracing::instrument(skip(self))]
    async fn commit(&mut self) -> Result<CommitResult, WorldStateError> {
        self.tx.commit().await
    }

    #[tracing::instrument(skip(self))]
    async fn rollback(&mut self) -> Result<(), WorldStateError> {
        self.tx.rollback().await
    }
}
