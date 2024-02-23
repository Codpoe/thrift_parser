use std::{
  collections::HashSet,
  env, fs,
  path::{Path, PathBuf},
  sync::{
    mpsc::{channel, Sender},
    Arc, Mutex,
  },
};

use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
  generate::{GenerateOptions, Generator},
  parse::Parser,
  visit::Visit,
};

pub struct Compiler {
  input: Vec<String>,
  src_dir: String,
  out_dir: String,
  options: GenerateOptions,
}

impl Compiler {
  pub fn new(
    input: Vec<String>,
    src_dir: String,
    out_dir: String,
    options: GenerateOptions,
  ) -> Self {
    Self {
      input,
      src_dir,
      out_dir,
      options,
    }
  }

  pub fn compile(&self) -> Result<(), String> {
    let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());
    let (err_sender, err_receiver) = channel::<String>();
    let seen = Arc::new(Mutex::new(vec![]));

    let src_dir = resolve_path(&self.src_dir).unwrap();
    let src_dir_path = Path::new(&src_dir);
    let out_dir: String = resolve_path(&self.out_dir).unwrap();
    let out_dir_path = Path::new(&out_dir);

    if out_dir_path.exists() {
      fs::remove_dir_all(out_dir_path).unwrap();
    }

    self.input.iter().for_each(|file| {
      let mut file = PathBuf::from(file);

      if file.is_relative() {
        file = src_dir_path.join(&file);
      }

      Self::compile_file(
        thread_pool.clone(),
        err_sender.clone(),
        seen.clone(),
        file.to_string_lossy().to_string(),
        src_dir.clone(),
        out_dir.clone(),
        self.options.clone(),
      );
    });

    drop(err_sender);

    if let Ok(err) = err_receiver.recv() {
      return Err(err);
    }

    Ok(())
  }

  fn compile_file(
    thread_pool: Arc<ThreadPool>,
    err_sender: Sender<String>,
    seen: Arc<Mutex<Vec<String>>>,
    file: String,
    src_dir: String,
    out_dir: String,
    options: GenerateOptions,
  ) {
    let cloned_thread_pool = thread_pool.clone();

    thread_pool.spawn(move || {
      let mut seen_data = seen.lock().unwrap();

      if seen_data.contains(&file) {
        return;
      }

      seen_data.push(file.clone());
      drop(seen_data);

      let code = fs::read_to_string(&file).unwrap();
      let mut relative_file = file.strip_prefix(&src_dir).unwrap();

      if relative_file.starts_with("/") {
        relative_file = relative_file.strip_prefix("/").unwrap();
      }

      // 解析 IDL 代码
      let mut ast = match Parser::new(&code).parse() {
        Ok(ast) => ast,
        Err(err) => {
          err_sender
            .send(format!("Compiler failed: {}. {}", relative_file, err))
            .unwrap();
          return;
        }
      };

      // 生成 TS 代码
      let ts_code = Generator::new(&mut ast).build(options.clone());

      // 写入文件
      let mut out_file = PathBuf::from(&out_dir).join(relative_file);
      out_file.set_extension("ts");
      fs::create_dir_all(out_file.parent().unwrap()).unwrap();
      fs::write(&out_file, ts_code).unwrap();

      // 分析依赖，继续解析
      let mut deps_visitor = DepsVisitor::new();
      deps_visitor.visit_document(&mut ast);

      for dep in deps_visitor.deps {
        let dep_file = Path::new(&file)
          .parent()
          .unwrap()
          .join(&dep)
          .to_string_lossy()
          .to_string();

        Self::compile_file(
          cloned_thread_pool.clone(),
          err_sender.clone(),
          seen.clone(),
          dep_file,
          src_dir.clone(),
          out_dir.clone(),
          options.clone(),
        );
      }
    });
  }
}

struct DepsVisitor {
  pub deps: HashSet<String>,
}

impl DepsVisitor {
  pub fn new() -> Self {
    Self {
      deps: HashSet::new(),
    }
  }
}

impl Visit for DepsVisitor {
  fn visit_include_definition(&mut self, include_definition: &mut crate::parse::IncludeDefinition) {
    self.deps.insert(include_definition.path.value.clone());
  }
}

fn resolve_path(mut path: &str) -> std::io::Result<String> {
  if Path::new(path).is_relative() {
    if path.starts_with("./") {
      path = path.strip_prefix("./").unwrap();
    }

    let current_dir = env::current_dir()?;
    let full_path = current_dir.join(path).to_string_lossy().to_string();
    Ok(full_path)
  } else {
    Ok(path.to_string())
  }
}
