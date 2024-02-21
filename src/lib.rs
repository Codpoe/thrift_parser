use nom::{
  branch::alt,
  bytes::complete::{is_not, tag, take_till, take_until},
  character::complete::{char, digit1, multispace0, space0, space1},
  combinator::{map, opt},
  error::context,
  multi::{many0, separated_list1},
  sequence::{delimited, preceded, separated_pair, tuple},
  IResult,
};

/// # thrift 基础语法
///
/// ```text
/// namespace x a.b.c
///
/// include "a.thrift"
///
/// struct GetDataReq {
///     // 这是单行注释
///     // 这也是单行注释
///     1: string parameters
///     /* 这是多行注释 */
///     2: i32 status (api.query="status")
///     3: double money
///     3: bool is_ok
///     2: optional map<string, string> kvs
///     3: required list<a.A> a_list
///     6: ItemType item_type
/// }
///
/// struct GetDataRes {
///     1: i32 status
///     2: string msg
/// }
///
/// enum ItemType {
///     // 未知
///     Unknown = 0
///     // 普通
///     Normal = 1
///     // 特别
///     Special = 2
/// }
///
/// service ThriftService {
///     GetDataRes GetData(1: GetDataReq req) (api.get = "/api/get-data", other = "something")
/// }
/// ```
#[derive(Debug)]
pub struct ThriftDocument {
  pub body: Vec<TopDefinition>,
}

#[derive(Debug)]
pub enum TopDefinition {
  Namespace(NamespaceDefinition),
  Include(IncludeDefinition),
  Struct(StructDefinition),
  Enum(EnumDefinition),
  Service(ServiceDefinition),
}

#[derive(Debug)]
pub struct NamespaceDefinition {
  pub scope: Identifier,
  pub name: Identifier,
}

#[derive(Debug)]
pub struct IncludeDefinition {
  pub path: StringLiteral,
}

#[derive(Debug)]
pub struct StructDefinition {
  pub name: Identifier,
  pub fields: Vec<FieldDefinition>,
  pub comments: Vec<Comment>,
}

#[derive(Debug)]
pub struct EnumDefinition {
  pub name: Identifier,
  pub members: Vec<EnumMember>,
  pub comments: Vec<Comment>,
}

#[derive(Debug)]
pub struct FieldDefinition {
  pub name: Identifier,
  pub field_id: FieldId,
  pub field_type: ThriftType,
  pub requiredness: Option<Requiredness>,
  pub comments: Vec<Comment>,
  pub annotations: Option<Annotations>,
}

#[derive(Debug)]
pub struct FieldId {
  pub value: usize,
}

#[derive(Debug, PartialEq)]
pub enum ThriftType {
  Void,
  String,
  I16,
  I32,
  I64,
  Double,
  Bool,
  List(Box<ThriftType>),
  Map(Box<ThriftType>, Box<ThriftType>),
  Identifier(Identifier),
}

#[derive(Debug)]
pub struct EnumMember {
  pub name: Identifier,
  pub initializer: Option<IntegerLiteral>,
  pub comments: Vec<Comment>,
}

#[derive(Debug, PartialEq)]
pub struct IntegerLiteral {
  pub value: String,
}

#[derive(Debug)]
pub struct ServiceDefinition {
  pub name: Identifier,
  pub functions: Vec<FunctionDefinition>,
  pub comments: Vec<Comment>,
}

#[derive(Debug)]
pub struct FunctionDefinition {
  pub name: Identifier,
  pub return_type: ThriftType,
  pub fields: Vec<FieldDefinition>,
  pub comments: Vec<Comment>,
  pub annotations: Option<Annotations>,
}

#[derive(Debug, PartialEq)]
pub struct Identifier {
  pub value: String,
}

#[derive(Debug, PartialEq)]
pub enum Requiredness {
  Optional,
  Required,
}

#[derive(Debug)]
pub enum Comment {
  Line(CommentLine),
  Block(CommentBlock),
}

impl Comment {
  pub fn line_value(&self) -> String {
    match self {
      Comment::Line(v) => v.value.clone(),
      Comment::Block(_) => panic!(),
    }
  }

  pub fn block_value(&self) -> Vec<String> {
    match self {
      Comment::Line(_) => panic!(),
      Comment::Block(v) => v.value.clone(),
    }
  }
}

#[derive(Debug)]
pub struct CommentLine {
  pub value: String,
}

#[derive(Debug)]
pub struct CommentBlock {
  pub value: Vec<String>,
}

#[derive(Debug)]
pub struct Annotations {
  pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub struct Annotation {
  pub name: Identifier,
  pub value: StringLiteral,
}

#[derive(Debug, PartialEq)]
pub struct StringLiteral {
  pub value: String,
}

fn identifier(i: &str) -> IResult<&str, Identifier> {
  context(
    "identifier",
    map(
      preceded(multispace0, is_not(" \t\n\r-=(){}[]<>")),
      |v: &str| Identifier {
        value: v.to_string(),
      },
    ),
  )(i)
}

fn string_literal(i: &str) -> IResult<&str, StringLiteral> {
  context(
    "string_literal",
    map(
      delimited(char('"'), take_till(|c: char| c == '\"'), char('"')),
      |v: &str| StringLiteral {
        value: v.to_string(),
      },
    ),
  )(i)
}

fn namespace_definition(i: &str) -> IResult<&str, NamespaceDefinition> {
  context(
    "namespace_definition",
    map(
      preceded(
        multispace0,
        tuple((tag("namespace"), space1, identifier, identifier)),
      ),
      |v| NamespaceDefinition {
        scope: v.2,
        name: v.3,
      },
    ),
  )(i)
}

fn include_definition(i: &str) -> IResult<&str, IncludeDefinition> {
  context(
    "include_definition",
    map(
      preceded(multispace0, tuple((tag("include"), space1, string_literal))),
      |v| IncludeDefinition { path: v.2 },
    ),
  )(i)
}

fn field_id(i: &str) -> IResult<&str, FieldId> {
  context(
    "field_id",
    map(delimited(multispace0, digit1, tag(":")), |v: &str| {
      FieldId {
        value: v.parse::<usize>().unwrap(),
      }
    }),
  )(i)
}

fn requiredness(i: &str) -> IResult<&str, Requiredness> {
  context(
    "requiredness",
    map(
      preceded(space1, alt((tag("optional"), tag("required")))),
      |v| match v {
        "optional" => Requiredness::Optional,
        "required" => Requiredness::Required,
        _ => unreachable!(),
      },
    ),
  )(i)
}

fn list_type(i: &str) -> IResult<&str, Box<ThriftType>> {
  context(
    "list_type",
    map(
      preceded(tag("list"), delimited(char('<'), thrift_type, char('>'))),
      |v| Box::new(v),
    ),
  )(i)
}

fn map_type(i: &str) -> IResult<&str, (Box<ThriftType>, Box<ThriftType>)> {
  context(
    "map_type",
    map(
      preceded(
        tag("map"),
        delimited(
          char('<'),
          separated_pair(
            thrift_type,
            delimited(space0, tag(","), space0),
            thrift_type,
          ),
          char('>'),
        ),
      ),
      |v| (Box::new(v.0), Box::new(v.1)),
    ),
  )(i)
}

fn thrift_type(i: &str) -> IResult<&str, ThriftType> {
  context(
    "field_type",
    preceded(
      space0,
      alt((
        map(tag("void"), |_| ThriftType::Void),
        map(tag("string"), |_| ThriftType::String),
        map(tag("i16"), |_| ThriftType::I16),
        map(tag("i32"), |_| ThriftType::I32),
        map(tag("i64"), |_| ThriftType::I64),
        map(tag("double"), |_| ThriftType::Double),
        map(tag("bool"), |_| ThriftType::Bool),
        map(list_type, |v| ThriftType::List(v)),
        map(map_type, |v| ThriftType::Map(v.0, v.1)),
        map(identifier, |v| ThriftType::Identifier(v)),
      )),
    ),
  )(i)
}

fn comment_line(i: &str) -> IResult<&str, CommentLine> {
  context(
    "comment_line",
    map(
      preceded(
        multispace0,
        preceded(tag("//"), take_till(|c| c == '\n' || c == '\r')),
      ),
      |v: &str| CommentLine {
        value: v.trim().to_string(),
      },
    ),
  )(i)
}

fn comment_block(i: &str) -> IResult<&str, CommentBlock> {
  context(
    "comment_block",
    map(
      preceded(
        multispace0,
        delimited(tag("/*"), take_until("*/"), tag("*/")),
      ),
      |v: &str| CommentBlock {
        value: v
          // .into_iter()
          // .collect::<String>()
          .trim()
          .split('\n')
          .map(|line| line.trim().to_string())
          .collect(),
      },
    ),
  )(i)
}

fn comment(i: &str) -> IResult<&str, Comment> {
  context(
    "comment",
    alt((
      map(comment_line, |v| Comment::Line(v)),
      map(comment_block, |v| Comment::Block(v)),
    )),
  )(i)
}

fn comment_inline(i: &str) -> IResult<&str, Comment> {
  context(
    "comment_inline",
    map(
      preceded(
        space0,
        preceded(tag("//"), take_till(|c| c == '\n' || c == '\r')),
      ),
      |v: &str| {
        Comment::Line(CommentLine {
          value: v.trim().to_string(),
        })
      },
    ),
  )(i)
}

fn annotation(i: &str) -> IResult<&str, Annotation> {
  context(
    "annotation",
    map(
      separated_pair(
        identifier,
        delimited(space0, tag("="), space0),
        string_literal,
      ),
      |v| Annotation {
        name: v.0,
        value: v.1,
      },
    ),
  )(i)
}

fn annotations(i: &str) -> IResult<&str, Annotations> {
  context(
    "annotations",
    map(
      preceded(
        space0,
        delimited(tag("("), separated_list1(tag(","), annotation), tag(")")),
      ),
      |v| Annotations { annotations: v },
    ),
  )(i)
}

fn field_definition(i: &str) -> IResult<&str, FieldDefinition> {
  context(
    "field_definition",
    map(
      tuple((
        many0(comment),
        field_id,
        opt(requiredness),
        thrift_type,
        identifier,
        opt(annotations),
        opt(comment_inline),
      )),
      |mut v| {
        if let Some(inline) = v.6 {
          v.0.push(inline);
        };

        FieldDefinition {
          field_id: v.1,
          requiredness: v.2,
          field_type: v.3,
          name: v.4,
          comments: v.0,
          annotations: v.5,
        }
      },
    ),
  )(i)
}

fn struct_definition_without_comments(i: &str) -> IResult<&str, StructDefinition> {
  context(
    "struct_definition",
    map(
      preceded(
        multispace0,
        preceded(
          tag("struct"),
          tuple((
            identifier,
            delimited(
              preceded(multispace0, tag("{")),
              many0(field_definition),
              preceded(multispace0, tag("}")),
            ),
          )),
        ),
      ),
      |v| StructDefinition {
        name: v.0,
        fields: v.1,
        comments: vec![],
      },
    ),
  )(i)
}

fn struct_definition(i: &str) -> IResult<&str, StructDefinition> {
  context(
    "struct_definition",
    map(
      preceded(
        multispace0,
        tuple((many0(comment), struct_definition_without_comments)),
      ),
      |v| StructDefinition {
        comments: v.0,
        ..v.1
      },
    ),
  )(i)
}

fn enum_member(i: &str) -> IResult<&str, EnumMember> {
  context(
    "enum_member",
    map(
      tuple((
        many0(comment),
        tuple((
          identifier,
          opt(preceded(delimited(space0, tag("="), space0), digit1)),
        )),
        opt(comment_inline),
      )),
      |mut v| {
        if let Some(inline) = v.2 {
          v.0.push(inline);
        }

        EnumMember {
          name: v.1 .0,
          initializer: v.1 .1.map(|str| IntegerLiteral {
            value: str.to_string(),
          }),
          comments: v.0,
        }
      },
    ),
  )(i)
}

fn enum_definition_without_comments(i: &str) -> IResult<&str, EnumDefinition> {
  context(
    "enum_definition",
    map(
      preceded(
        multispace0,
        preceded(
          tag("enum"),
          tuple((
            identifier,
            delimited(
              preceded(multispace0, tag("{")),
              many0(enum_member),
              preceded(multispace0, tag("}")),
            ),
          )),
        ),
      ),
      |v| EnumDefinition {
        name: v.0,
        members: v.1,
        comments: vec![],
      },
    ),
  )(i)
}

fn enum_definition(i: &str) -> IResult<&str, EnumDefinition> {
  context(
    "enum_definition",
    map(
      preceded(
        multispace0,
        tuple((many0(comment), enum_definition_without_comments)),
      ),
      |v| EnumDefinition {
        comments: v.0,
        ..v.1
      },
    ),
  )(i)
}

fn function_definition_without_comments(i: &str) -> IResult<&str, FunctionDefinition> {
  context(
    "function_definition",
    map(
      preceded(
        multispace0,
        tuple((
          thrift_type,
          identifier,
          delimited(
            preceded(space0, tag("(")),
            many0(field_definition),
            tag(")"),
          ),
          opt(annotations),
        )),
      ),
      |v| FunctionDefinition {
        name: v.1,
        return_type: v.0,
        fields: v.2,
        comments: vec![],
        annotations: v.3,
      },
    ),
  )(i)
}

fn function_definition(i: &str) -> IResult<&str, FunctionDefinition> {
  context(
    "function_definition",
    map(
      preceded(
        multispace0,
        tuple((many0(comment), function_definition_without_comments)),
      ),
      |v| FunctionDefinition {
        comments: v.0,
        ..v.1
      },
    ),
  )(i)
}

fn service_definition_without_comments(i: &str) -> IResult<&str, ServiceDefinition> {
  context(
    "service_definition",
    map(
      preceded(
        multispace0,
        preceded(
          tag("service"),
          tuple((
            identifier,
            delimited(
              preceded(multispace0, tag("{")),
              many0(function_definition),
              preceded(multispace0, tag("}")),
            ),
          )),
        ),
      ),
      |v| ServiceDefinition {
        name: v.0,
        functions: v.1,
        comments: vec![],
      },
    ),
  )(i)
}

fn service_definition(i: &str) -> IResult<&str, ServiceDefinition> {
  context(
    "service_definition",
    map(
      preceded(
        multispace0,
        tuple((many0(comment), service_definition_without_comments)),
      ),
      |v| ServiceDefinition {
        comments: v.0,
        ..v.1
      },
    ),
  )(i)
}

pub fn parse_thrift_document(i: &str) -> IResult<&str, ThriftDocument> {
  context(
    "thrift_document",
    map(
      many0(alt((
        map(namespace_definition, |v| TopDefinition::Namespace(v)),
        map(include_definition, |v| TopDefinition::Include(v)),
        map(struct_definition, |v| TopDefinition::Struct(v)),
        map(enum_definition, |v| TopDefinition::Enum(v)),
        map(service_definition, |v| TopDefinition::Service(v)),
      ))),
      |v| ThriftDocument { body: v },
    ),
  )(i)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_string_literal() {
    let (_, ret) = string_literal("\"hello\"").unwrap();
    assert_eq!(ret.value, "hello");
  }

  #[test]
  fn test_identifier() {
    let (_, ret) = identifier("hello a").unwrap();
    assert_eq!(ret.value, "hello");
  }

  #[test]
  fn test_namespace_definition() {
    let (_, ret) = namespace_definition(" namespace x a.b.c ").unwrap();
    println!("{:?}", ret);
    assert_eq!(ret.scope.value, "x");
    assert_eq!(ret.name.value, "a.b.c");
  }

  #[test]
  fn test_include_definition() {
    let (_, ret) = include_definition(r#" include "a.thrift" "#).unwrap();
    println!("{:?}", ret);
    assert_eq!(ret.path.value, "a.thrift");
  }

  #[test]
  fn test_field_id() {
    let (_, ret) = field_id(" 1: string parameters").unwrap();
    println!("{:?}", ret);
    assert_eq!(ret.value, 1);
  }

  #[test]
  fn test_requiredness() {
    let (_, ret) = requiredness(" optional").unwrap();
    println!("{:?}", ret);
    assert_eq!(ret, Requiredness::Optional);

    let (_, ret) = requiredness(" required").unwrap();
    println!("{:?}", ret);
    assert_eq!(ret, Requiredness::Required);
  }

  #[test]
  fn test_comment() {
    let (_, ret) = comment("// hello").unwrap();
    println!("{:?}", ret);

    match ret {
      Comment::Line(line) => assert_eq!(line.value, "hello"),
      _ => panic!(),
    }

    let (_, ret) = comment("/* hello\n world */").unwrap();
    println!("{:?}", ret);

    match ret {
      Comment::Block(block) => assert_eq!(block.value, ["hello", "world"]),
      _ => panic!(),
    }
  }

  #[test]
  fn test_annotations() {
    let (_, ret) = annotation("hello = \"world\"").unwrap();
    println!("{:?}", ret);
    assert_eq!(ret.name.value, "hello");
    assert_eq!(ret.value.value, "world");

    let (_, ret) = annotations(" (hello = \"world\", some = \"thing\")").unwrap();
    println!("{:?}", ret);
    assert_eq!(ret.annotations.len(), 2);
    assert_eq!(ret.annotations[0].name.value, "hello");
    assert_eq!(ret.annotations[0].value.value, "world");
    assert_eq!(ret.annotations[1].name.value, "some");
    assert_eq!(ret.annotations[1].value.value, "thing");
  }

  #[test]
  fn test_thrift_type() {
    let (_, ret) = thrift_type("string").unwrap();
    assert_eq!(ret, ThriftType::String);

    let (_, ret) = thrift_type("list<a.A>").unwrap();
    assert_eq!(
      ret,
      ThriftType::List(Box::new(ThriftType::Identifier(Identifier {
        value: "a.A".to_string()
      })))
    );

    let (_, ret) = thrift_type("map<string, string>").unwrap();
    assert_eq!(
      ret,
      ThriftType::Map(Box::new(ThriftType::String), Box::new(ThriftType::String))
    );
  }

  #[test]
  fn test_field_definition() {
    let (_, ret) = field_definition(
      " // 这是单行注释\n // 这也是单行注释\n /* 这是多行注释 */\n 1: optional string parameters (api.query = \"param\") // 行后的注释",
    )
    .unwrap();
    println!("{:?}", ret);

    assert_eq!(ret.comments.len(), 4);
    assert_eq!(ret.comments[0].line_value(), "这是单行注释");
    assert_eq!(ret.comments[1].line_value(), "这也是单行注释");
    assert_eq!(ret.comments[2].block_value(), ["这是多行注释"]);
    assert_eq!(ret.comments[3].line_value(), "行后的注释");
    assert_eq!(ret.field_id.value, 1);
    assert_eq!(ret.requiredness.unwrap(), Requiredness::Optional);
    assert_eq!(ret.field_type, ThriftType::String);
    assert_eq!(ret.name.value, "parameters");
    assert_eq!(ret.annotations.as_ref().unwrap().annotations.len(), 1);
    assert_eq!(
      ret.annotations.as_ref().unwrap().annotations[0].name.value,
      "api.query"
    );
    assert_eq!(
      ret.annotations.as_ref().unwrap().annotations[0].value.value,
      "param"
    );
  }

  #[test]
  fn test_struct_definition() {
    let (_, ret) = struct_definition(
      r#"
// 这是 struct 注释 1
// 这是 struct 注释 2
struct GetDataReq {
  // 这是单行注释
  // 这也是单行注释
  1: string parameters
  /* 这是多行注释 */
  2: i32 status (api.query="status")
  3: double money
  4: bool is_ok
  5: optional map<string, string> kvs // 行内注释
  6: required list<a.A> a_list
  7: ItemType item_type
}
"#,
    )
    .unwrap();
    println!("{:?}", ret);

    assert_eq!(ret.comments.len(), 2);
    assert_eq!(ret.comments[0].line_value(), "这是 struct 注释 1");
    assert_eq!(ret.comments[1].line_value(), "这是 struct 注释 2");
    assert_eq!(ret.name.value, "GetDataReq");
    assert_eq!(ret.fields.len(), 7);
    assert_eq!(ret.fields[4].comments.len(), 1);
    assert_eq!(ret.fields[4].comments[0].line_value(), "行内注释");
    assert_eq!(ret.fields[4].field_id.value, 5);
    assert_eq!(
      ret.fields[4].requiredness.as_ref().unwrap(),
      &Requiredness::Optional
    );
    assert_eq!(
      ret.fields[4].field_type,
      ThriftType::Map(Box::new(ThriftType::String), Box::new(ThriftType::String))
    );
    assert_eq!(ret.fields[4].name.value, "kvs");
  }

  #[test]
  fn test_enum_member() {
    let (_, ret) = enum_member("// 注释\n Unknown = 0").unwrap();
    println!("{:?}", ret);
  }

  #[test]
  fn test_enum_definition() {
    let (_, ret) = enum_definition(
      r#"
// 这是 enum 注释
enum ItemType {
  // 未知
  Unknown = 0
  // 普通
  Normal = 1
  // 特别
  Special = 2
}
"#,
    )
    .unwrap();
    println!("{:?}", ret);

    assert_eq!(ret.comments.len(), 1);
    assert_eq!(ret.comments[0].line_value(), "这是 enum 注释");
    assert_eq!(ret.name.value, "ItemType");
    assert_eq!(ret.members.len(), 3);
    assert_eq!(ret.members[0].comments.len(), 1);
    assert_eq!(ret.members[0].comments[0].line_value(), "未知");
    assert_eq!(ret.members[0].name.value, "Unknown");
    assert_eq!(ret.members[0].initializer.as_ref().unwrap().value, "0");
  }

  #[test]
  fn test_function_definition() {
    let (_, ret) = function_definition(
      r#"
    // 这是函数注释
    GetDataRes GetData(1: GetDataReq req) (api.get = "/api/get-data", other = "something")
"#,
    )
    .unwrap();
    println!("{:?}", ret);
  }

  #[test]
  fn test_service_definition() {
    let (_, ret) = service_definition(
      r#"
// 这是 service 注释
service ThriftService {
  // 这是函数注释
  GetDataRes GetData(1: GetDataReq req) (api.get = "/api/get-data", other = "something")
}
"#,
    )
    .unwrap();
    println!("{:?}", ret);

    assert_eq!(ret.comments.len(), 1);
    assert_eq!(ret.comments[0].line_value(), "这是 service 注释");
    assert_eq!(ret.name.value, "ThriftService");
    assert_eq!(ret.functions.len(), 1);
    assert_eq!(ret.functions[0].comments[0].line_value(), "这是函数注释");
    assert_eq!(ret.functions[0].name.value, "GetData");
    assert_eq!(
      ret.functions[0].return_type,
      ThriftType::Identifier(Identifier {
        value: "GetDataRes".to_string()
      })
    );
    assert_eq!(ret.functions[0].fields.len(), 1);
    assert_eq!(ret.functions[0].fields[0].name.value, "req");
    assert_eq!(
      ret.functions[0].fields[0].field_type,
      ThriftType::Identifier(Identifier {
        value: "GetDataReq".to_string()
      })
    );
    assert_eq!(
      ret.functions[0]
        .annotations
        .as_ref()
        .unwrap()
        .annotations
        .len(),
      2
    );
  }

  #[test]
  fn it_works() {
    let ret = parse_thrift_document(
      r#"
namespace x a.b.c

include "a.thrift"

struct GetDataReq {
    // 这是单行注释
    // 这也是单行注释
    1: string parameters
    /* 这是多行注释 */
    2: i32 status (api.query="status")
    3: double money
    3: bool is_ok
    2: optional map<string, string> kvs
    3: required list<a.A> a_list
    6: ItemType item_type
}

struct GetDataRes {
    1: i32 status
    2: string msg
}

enum ItemType {
    // 未知
    Unknown = 0
    // 普通
    Normal = 1
    // 特别
    Special = 2
}

service ThriftService {
    GetDataRes GetData(1: GetDataReq req) (api.get = "/api/get-data", other = "something")
}
"#,
    )
    .unwrap();

    println!("{:?}", ret);
  }
}
