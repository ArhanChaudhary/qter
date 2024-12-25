use mlua::{AnyUserData, IntoLua, Lua, UserDataMethods, UserDataRegistry, Value};
use qter_core::{Int, I};

pub struct LuaMacros {
    lua: Lua,
}

impl LuaMacros {
    pub fn new() -> mlua::Result<LuaMacros> {
        let lua = Lua::new();

        lua.register_userdata_type(Self::int_userdata)?;

        let big = lua.create_function(|_lua, v| Ok(AnyUserData::wrap(Self::value_to_int(v)?)))?;

        lua.globals().set("big", big)?;

        Ok(LuaMacros { lua })
    }

    pub fn add_chunk(&self, code: &str) -> mlua::Result<()> {
        self.lua.load(code).exec()
    }

    fn value_to_int(v: Value) -> mlua::Result<Int<I>> {
        match v {
            Value::Integer(int) => Ok(Int::from(int)),
            Value::UserData(data) => data.borrow::<Int<I>>().map(|v| *v),
            _ => Err(mlua::Error::runtime("The value isn't an integer!")),
        }
    }

    fn int_userdata(registry: &mut UserDataRegistry<Int<I>>) {
        registry.add_meta_function("__add", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(AnyUserData::wrap(
                Self::value_to_int(lhs)? + Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__sub", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(AnyUserData::wrap(
                Self::value_to_int(lhs)? - Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__mul", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(AnyUserData::wrap(
                Self::value_to_int(lhs)? * Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__div", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(AnyUserData::wrap(
                Self::value_to_int(lhs)? / Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__mod", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(AnyUserData::wrap(Int::<I>::from(
                Self::value_to_int(lhs)? % Self::value_to_int(rhs)?,
            )))
        });

        registry.add_meta_function("__unm", |_lua, v: Value| {
            Ok(AnyUserData::wrap(-Self::value_to_int(v)?))
        });

        registry.add_meta_function("__eq", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(Value::Boolean(
                Self::value_to_int(lhs)? == Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__lt", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(Value::Boolean(
                Self::value_to_int(lhs)? < Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__le", |_lua, (lhs, rhs): (Value, Value)| {
            Ok(Value::Boolean(
                Self::value_to_int(lhs)? <= Self::value_to_int(rhs)?,
            ))
        });

        registry.add_meta_function("__tostring", |lua, v: Value| {
            mlua::String::wrap(Self::value_to_int(v)?.to_string().as_bytes()).into_lua(lua)
        });
    }
}

#[cfg(test)]
mod tests {
    use mlua::{AnyUserData, Function};
    use qter_core::{Int, I};

    use super::LuaMacros;

    #[test]
    fn custom_numeric() {
        let lua = LuaMacros::new().unwrap();

        lua.add_chunk(
            "
            function fail()
                assert(false)
            end

            function test(zero, too_big, tenth_too_big)
                assert(zero < big(10))
                assert(zero + 10 <= big(10))
                assert(too_big / 10 == tenth_too_big)
                assert(too_big % 9 == big(1))
                assert(10 / big(6) == big(1))
                assert(10 - big(4) == big(6))
                assert(-big(10) == big(-10))
            end
        ",
        )
        .unwrap();

        assert!(lua
            .lua
            .globals()
            .get::<Function>("fail")
            .unwrap()
            .call::<()>(())
            .is_err());

        let too_big = Int::<I>::from(u64::MAX - 5);
        let too_big = too_big * too_big;

        lua.lua
            .globals()
            .get::<Function>("test")
            .unwrap()
            .call::<()>((
                AnyUserData::wrap(Int::<I>::zero()),
                AnyUserData::wrap(too_big),
                AnyUserData::wrap(too_big / Int::<I>::from(10)),
            ))
            .unwrap();
    }
}
