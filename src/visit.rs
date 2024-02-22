use crate::parse::{
  EnumDefinition, EnumMember, FieldDefinition, FunctionDefinition, IncludeDefinition,
  NamespaceDefinition, ServiceDefinition, StructDefinition, ThriftDocument, TopDefinition,
};

pub trait Visit {
  fn visit_document(&mut self, document: &mut ThriftDocument) {
    for definition in &mut document.body {
      match definition {
        TopDefinition::Namespace(namespace_definition) => {
          self.visit_namespace_definition(namespace_definition)
        }
        TopDefinition::Include(include_definition) => {
          self.visit_include_definition(include_definition)
        }
        TopDefinition::Struct(struct_definition) => self.visit_struct_definition(struct_definition),
        TopDefinition::Enum(enum_definition) => self.visit_enum_definition(enum_definition),
        TopDefinition::Service(service_definition) => {
          self.visit_service_definition(service_definition)
        }
      }
    }
  }

  fn visit_namespace_definition(&mut self, namespace_definition: &mut NamespaceDefinition) {}

  fn visit_include_definition(&mut self, include_definition: &mut IncludeDefinition) {}

  fn visit_struct_definition(&mut self, struct_definition: &mut StructDefinition) {
    for field_definition in &mut struct_definition.fields {
      self.visit_struct_field_definition(field_definition)
    }
  }

  fn visit_enum_definition(&mut self, enum_definition: &mut EnumDefinition) {
    for enum_member in &mut enum_definition.members {
      self.visit_enum_member(enum_member)
    }
  }

  fn visit_service_definition(&mut self, service_definition: &mut ServiceDefinition) {
    for function_definition in &mut service_definition.functions {
      self.visit_function_definition(function_definition)
    }
  }

  fn visit_function_definition(&mut self, function_definition: &mut FunctionDefinition) {}

  fn visit_struct_field_definition(&mut self, field_definition: &mut FieldDefinition) {}

  fn visit_enum_member(&mut self, enum_member: &mut EnumMember) {}
}
