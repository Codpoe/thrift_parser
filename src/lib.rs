pub mod generate;
pub mod parse;
pub mod visit;

#[cfg(test)]
mod tests {
  use crate::{
    generate::{GenerateOptions, Generator},
    parse::Parser,
  };

  #[test]
  fn it_works() {
    let idl = r#"
namespace x a.b.c
    
include "a.thrift"
    
struct GetDataReq {
    // 这是单行注释
    // 这也是单行注释
    1: string parameters
    /* 这是多行注释 */
    2: i32 status (api.query="query_status")
    3: double money
    3: bool is_ok
    2: optional map<a.A, string> kvs
    3: required list<a.A> a_list
    6: ItemType item_type
}
    
struct GetDataRes {
    1: i32 status (api.body="body_status")
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
    // 获取数据
    GetDataRes GetData(1: GetDataReq req) (api.get = "/api/get-data", other = "something")
}
"#;
    let mut thrift_document = Parser::new(idl).parse();

    std::fs::write("./tests/fixtures/ast", format!("{:#?}", thrift_document)).unwrap();

    let ts_code = Generator::new(&mut thrift_document).build(GenerateOptions::default());

    std::fs::write("./tests/fixtures/gen.ts", ts_code).unwrap();
  }
}
