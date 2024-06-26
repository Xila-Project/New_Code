#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use wamr_rust_sdk::{function::Function, instance::Instance, value::WasmValue};

use crate::{
    Data::Data_type, Environment_type, Module::Module_type, Result_type, Runtime::Runtime_type,
};

pub struct Instance_type(Instance);

impl Instance_type {
    pub fn New(
        Runtime: &Runtime_type,
        Module: &Module_type,
        Stack_size: usize,
        Data: &Data_type,
    ) -> Result_type<Self> {
        let Instance = Instance_type(Instance::new(
            Runtime.Get_inner_reference(),
            Module.Get_inner_reference(),
            Stack_size as u32,
        )?);

        let mut Execution_environment = Environment_type::From_instance(&Instance)?;

        Execution_environment.Set_user_data(Data);

        Ok(Instance)
    }

    pub fn Call_export_function(
        &self,
        Name: &str,
        Parameters: &Vec<WasmValue>,
    ) -> Result_type<WasmValue> {
        if Parameters.is_empty() {
            Ok(
                Function::find_export_func(self.Get_inner_reference(), Name)?
                    .call(&self.0, &vec![WasmValue::I32(0)])?,
            )
        } else {
            Ok(
                Function::find_export_func(self.Get_inner_reference(), Name)?
                    .call(&self.0, Parameters)?,
            )
        }
    }

    pub fn Call_main(&self, Parameters: &Vec<WasmValue>) -> Result_type<WasmValue> {
        self.Call_export_function("main", Parameters)
    }

    pub(crate) fn Get_inner_reference(&self) -> &Instance {
        &self.0
    }
}
