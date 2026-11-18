use std::collections::HashMap;
use std::{cell::RefCell, fmt::Display, iter::Peekable, rc::Rc, vec::IntoIter};

use crate::{
    error_handler::Errors,
    ownership::{BorrowChecker, CheckedEnvironment},
    tokens::{LiteralType, Token, TokenType},
};
// LoxClass and LoxInstance for OOP support
// Keeping a direct AST interpreter alongside transpilation lets me run fast sandbox checks
// and catch runtime errors before burning a full `cargo build` cycle.
#[derive(Debug, Clone)]
pub struct LoxClass {
    pub name: String,
    pub methods: HashMap<String, LoxFunction>,
    pub superclass: Option<Rc<LoxClass>>,
}

#[derive(Debug, Clone)]
pub struct LoxInstance {
    pub class: Rc<LoxClass>,
    pub fields: RefCell<HashMap<String, DataTypes>>,
}

#[derive(Debug, Clone)]
pub struct LoxFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Statment>,
    pub closure: Rc<RefCell<Environment>>,
    pub is_initializer: bool,
}

impl LoxClass {
    pub fn new(
        name: String,
        methods: HashMap<String, LoxFunction>,
        superclass: Option<Rc<LoxClass>>,
    ) -> Self {
        LoxClass {
            name,
            methods,
            superclass,
        }
    }

    pub fn find_method(&self, name: &str) -> Option<LoxFunction> {
        if let Some(method) = self.methods.get(name) {
            return Some(method.clone());
        }
        if let Some(superclass) = &self.superclass {
            return superclass.find_method(name);
        }
        None
    }
}

impl LoxInstance {
    pub fn new(class: Rc<LoxClass>) -> Self {
        LoxInstance {
            class,
            fields: RefCell::new(HashMap::new()),
        }
    }

    pub fn get(&self, name: &str) -> Option<DataTypes> {
        if let Some(value) = self.fields.borrow().get(name) {
            return Some(value.clone());
        }
        if let Some(method) = self.class.find_method(name) {
            return Some(DataTypes::Function {
                params: method.params,
                body: method.body,
                capture: method.closure,
            });
        }
        None
    }

    pub fn set(&self, name: String, value: DataTypes) {
        self.fields.borrow_mut().insert(name, value);
    }
}

#[derive(Debug, Clone)]
pub enum DataTypes {
    Number(f32),
    Bool(bool),
    Str(String),
    Array(Vec<DataTypes>),
    Object(std::collections::HashMap<String, DataTypes>),
    Function {
        params: Vec<String>,
        body: Box<Statment>,
        capture: Rc<RefCell<Environment>>,
    },
    Class(Rc<LoxClass>),
    Instance(Rc<LoxInstance>),
    Result(Result<Box<DataTypes>, Box<DataTypes>>),
}
impl Display for DataTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataTypes::Number(num) => {
                let rounded = (*num * 10000.0).round() / 10000.0;
                if (rounded - rounded.round()).abs() < 1e-4 {
                    write!(f, "{}", rounded.round() as i64)
                } else {
                    write!(f, "{rounded}")
                }
            }
            DataTypes::Bool(b) => write!(f, "{b}"),
            DataTypes::Str(s) => write!(f, "\"{s}\""),
            DataTypes::Array(arr) => write!(
                f,
                "[{}]",
                arr.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            DataTypes::Object(obj) => write!(
                f,
                "{{{}}}",
                obj.iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            DataTypes::Function { params, .. } => write!(f, "<fn {}>", params.join(", ")),
            DataTypes::Class(class) => write!(f, "<class {}>", class.name),
            DataTypes::Instance(instance) => write!(f, "<instance of {}>", instance.class.name),
            DataTypes::Result(r) => match r {
                Ok(v) => write!(f, "Ok({v})"),
                Err(e) => write!(f, "Err({e})"),
            },
        }
    }
}

impl PartialEq for DataTypes {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DataTypes::Number(a), DataTypes::Number(b)) => a == b,
            (DataTypes::Bool(a), DataTypes::Bool(b)) => a == b,
            (DataTypes::Str(a), DataTypes::Str(b)) => a == b,
            (DataTypes::Array(a), DataTypes::Array(b)) => a == b,
            (DataTypes::Object(a), DataTypes::Object(b)) => a == b,
            (DataTypes::Class(a), DataTypes::Class(b)) => a.name == b.name,
            (DataTypes::Instance(a), DataTypes::Instance(b)) => a.class.name == b.class.name,
            _ => false,
        }
    }
}

impl From<DataTypes> for f32 {
    fn from(value: DataTypes) -> Self {
        match value {
            DataTypes::Number(num) => num,
            DataTypes::Bool(true) => 1.0,
            DataTypes::Bool(false) => 0.0,
            DataTypes::Str(_) => 0.0,
            DataTypes::Array(_) => 0.0,
            DataTypes::Object(_) => 0.0,
            DataTypes::Function { .. } => 0.0,
            DataTypes::Class(_) => 0.0,
            DataTypes::Instance(_) => 0.0,
            DataTypes::Result(_) => 0.0,
        }
    }
}
impl From<f32> for DataTypes {
    fn from(value: f32) -> Self {
        DataTypes::Number(value)
    }
}
impl From<DataTypes> for bool {
    fn from(value: DataTypes) -> Self {
        match value {
            DataTypes::Bool(b) => b,
            DataTypes::Number(0.0) => false,
            DataTypes::Number(_) => true,
            DataTypes::Str(_) => false,
            DataTypes::Array(arr) => !arr.is_empty(),
            DataTypes::Object(obj) => !obj.is_empty(),
            DataTypes::Function { .. } => true,
            DataTypes::Class(_) => true,
            DataTypes::Instance(_) => true,
            DataTypes::Result(r) => r.is_ok(),
        }
    }
}

impl From<bool> for DataTypes {
    fn from(value: bool) -> Self {
        DataTypes::Bool(value)
    }
}
impl From<Option<LiteralType>> for DataTypes {
    fn from(value: Option<LiteralType>) -> Self {
        match value {
            Some(LiteralType::Number(num)) => DataTypes::Number(num),
            Some(LiteralType::String(s)) => DataTypes::Str(s),
            Some(LiteralType::Bool(b)) => DataTypes::Bool(b),
            Some(LiteralType::Null) => DataTypes::Number(0.0),
            None => DataTypes::Number(0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Binary(Box<Expression>, Token, Box<Expression>),
    Unary(Token, Box<Expression>),
    Literal(Token),
    Grouping(Box<Expression>),
    Assign(String, Box<Expression>),
    Call(String, Vec<Expression>), // function name, arguments
    Array(Vec<Expression>),
    Index(Box<Expression>, Box<Expression>),
    IndexSet(Box<Expression>, Box<Expression>, Box<Expression>),
    Object(Vec<(String, Expression)>),
    Get(Box<Expression>, String),
    Set(Box<Expression>, String, Box<Expression>),
    This,
    Super(String),                      // super.method()
    SuperCall(String, Vec<Expression>), // super.method(args)
    Function(Vec<String>, Box<Statment>),
    Return(Option<Box<Expression>>),
}

// let var:Option

// visitor pattern for better maintainability
pub trait ExpressionVisitor<T> {
    fn visit_binary(&mut self, left: &Expression, operator: &Token, right: &Expression) -> T;
    fn visit_unary(&mut self, operator: &Token, right: &Expression) -> T;
    fn visit_literal(&mut self, token: &Token) -> T;
    fn visit_grouping(&mut self, expression: &Expression) -> T;
    fn visit_assign(&mut self, name: &str, value: &Expression) -> T;
    fn visit_call(&mut self, name: &str, args: &[Expression]) -> T;
    fn visit_array(&mut self, elements: &[Expression]) -> T;
    fn visit_index(&mut self, object: &Expression, index: &Expression) -> T;
    fn visit_index_set(&mut self, object: &Expression, index: &Expression, value: &Expression) -> T;
    fn visit_object(&mut self, fields: &[(String, Expression)]) -> T;
    fn visit_get(&mut self, object: &Expression, name: &str) -> T;
    fn visit_set(&mut self, object: &Expression, name: &str, value: &Expression) -> T;
    fn visit_this(&mut self) -> T;
    fn visit_super(&mut self, method: &str) -> T;
    fn visit_super_call(&mut self, method: &str, args: &[Expression]) -> T;
    fn visit_function(&mut self, params: &[String], body: &Statment) -> T;
    fn visit_return(&mut self, value: &Option<Box<Expression>>) -> T;
}
//operations as classes

#[derive(Debug, Clone)]
pub struct Environment {
    vars: HashMap<String, DataTypes>,
    parent: Option<Rc<RefCell<Environment>>>,
    pub return_val: Option<DataTypes>,
    pub assertion_failures: Vec<String>,
}

impl Environment {
    pub fn new(vars: HashMap<String, DataTypes>) -> Self {
        Environment {
            vars,
            parent: None,
            return_val: None,
            assertion_failures: Vec::new(),
        }
    }

    pub fn with_parent(vars: HashMap<String, DataTypes>, parent: Rc<RefCell<Environment>>) -> Self {
        Environment {
            vars,
            parent: Some(parent),
            return_val: None,
            assertion_failures: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<DataTypes> {
        if let Some(v) = self.vars.get(name) {
            Some(v.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get(name)
        } else {
            None
        }
    }

    pub fn set(&mut self, name: String, val: DataTypes) {
        if !self.vars.contains_key(&name) {
            if let Some(parent) = &self.parent {
                if parent.borrow().get(&name).is_some() {
                    parent.borrow_mut().set(name, val);
                    return;
                }
            }
        }
        self.vars.insert(name, val);
    }

    pub fn define(&mut self, name: String, val: DataTypes) {
        self.vars.insert(name, val);
    }
}

pub struct Interpret {
    pub environment: Rc<RefCell<Environment>>,
}

impl Default for Interpret {
    fn default() -> Self {
        Interpret {
            environment: Rc::new(RefCell::new(Environment::new(HashMap::new()))),
        }
    }
}

pub struct CheckedInterpret {
    pub environment: Rc<RefCell<CheckedEnvironment>>,
    pub check_borrows: bool,
}

impl Default for CheckedInterpret {
    fn default() -> Self {
        let borrow_checker = Rc::new(RefCell::new(BorrowChecker::new()));
        CheckedInterpret {
            environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
            check_borrows: false,
        }
    }
}

impl CheckedInterpret {
    pub fn execute(&mut self, stmt: &Statment) -> Option<DataTypes> {
        if !self.check_borrows {
            let mut interpreter = Interpret::default();
            return interpreter.execute(stmt);
        }
        self.execute_with_borrows(stmt)
    }

    fn execute_with_borrows(&mut self, stmt: &Statment) -> Option<DataTypes> {
        if let Some(ret) = self.environment.borrow().return_val.clone() {
            return Some(ret);
        }
        match stmt {
            Statment::ExpressionStatement(expr) => Some(self.visit_expr(expr)),
            Statment::PrintStatement(expr) => {
                let val = self.visit_expr(expr);
                match &val {
                    DataTypes::Str(s) if s == "__tui_rendered__" => {}
                    DataTypes::Str(s) => println!("{s}"),
                    other => println!("{other}"),
                }
                Some(val)
            }
            Statment::LetStatement(name, initializer) => {
                let val = self.visit_expr(initializer);
                let mut env = self.environment.borrow_mut();
                if self.check_borrows {
                    if let Expression::Literal(src_tok) = initializer {
                        if src_tok.token_type == TokenType::IDENTIFIER
                            && matches!(
                                val,
                                DataTypes::Array(_) | DataTypes::Object(_) | DataTypes::Instance(_)
                            )
                        {
                            if let Err(e) =
                                env.check_move(&src_tok.text, format!("move into {}", name.text))
                            {
                                let msg = format!("Borrow check error: {e}");
                                eprintln!("{msg}");
                                env.record_error(msg);
                            }
                        }
                    }
                }
                match env.declare(name.text.clone(), val.clone()) {
                    Ok(_) => Some(val),
                    Err(e) => {
                        let msg = format!("Borrow check error: {e}");
                        eprintln!("{msg}");
                        env.record_error(msg);
                        None
                    }
                }
            }
            Statment::IfStatement(condition, then_branch, else_branch) => {
                let cond_val = self.visit_expr(condition);
                let is_truthy = match cond_val {
                    DataTypes::Bool(b) => b,
                    DataTypes::Number(n) => n != 0.0,
                    DataTypes::Str(s) => !s.is_empty(),
                    _ => false,
                };
                if is_truthy {
                    self.execute_with_borrows(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.execute_with_borrows(else_stmt)
                } else {
                    None
                }
            }
            Statment::WhileStatement(condition, body) => {
                loop {
                    let cond_val = self.visit_expr(condition);
                    let is_truthy = match cond_val {
                        DataTypes::Bool(b) => b,
                        DataTypes::Number(n) => n != 0.0,
                        DataTypes::Str(s) => !s.is_empty(),
                        _ => false,
                    };
                    if !is_truthy {
                        break;
                    }
                    let mut env = self.environment.borrow_mut();
                    env.push_scope();
                    drop(env);
                    self.execute_with_borrows(body);
                    let mut env = self.environment.borrow_mut();
                    env.pop_scope();
                    if let Some(ret) = env.return_val.clone() {
                        return Some(ret);
                    }
                }
                None
            }
            Statment::ForStatement(init, condition, increment, body) => {
                if let Some(init_stmt) = init {
                    self.execute_with_borrows(init_stmt);
                }
                loop {
                    let should_continue = if let Some(cond_expr) = condition {
                        let cond_val = self.visit_expr(cond_expr);
                        match cond_val {
                            DataTypes::Bool(b) => b,
                            DataTypes::Number(n) => n != 0.0,
                            DataTypes::Str(s) => !s.is_empty(),
                            _ => false,
                        }
                    } else {
                        true
                    };
                    if !should_continue {
                        break;
                    }
                    let mut env = self.environment.borrow_mut();
                    env.push_scope();
                    drop(env);
                    self.execute_with_borrows(body);
                    if let Some(inc_expr) = increment {
                        self.visit_expr(inc_expr);
                    }
                    let mut env = self.environment.borrow_mut();
                    env.pop_scope();
                }
                None
            }
            Statment::BlockStatement(statements) => {
                let mut env = self.environment.borrow_mut();
                env.push_scope();
                drop(env);
                let mut last_val = None;
                for stmt in statements {
                    if let Some(v) = self.execute_with_borrows(stmt) {
                        last_val = Some(v)
                    }
                    if self.environment.borrow().return_val.is_some() {
                        break;
                    }
                }
                let mut env = self.environment.borrow_mut();
                env.pop_scope();
                last_val
            }
            Statment::ClassStatement(name_token, methods, _superclass) => {
                let class_name = name_token.text.clone();
                let mut class_methods = std::collections::HashMap::new();
                // Convert CheckedEnvironment to regular Environment for closure capture
                let parent_env = Environment::new(self.environment.borrow().vars.clone());
                for method_stmt in methods {
                    let (method_name, params, body) =
                        if let Statment::ExpressionStatement(Expression::Function(params, body)) =
                            method_stmt
                        {
                            let method_name = params.first().cloned().unwrap_or_default();
                            (method_name, params.clone(), body.clone())
                        } else if let Statment::ExpressionStatement(Expression::Assign(
                            name,
                            expr,
                        )) = method_stmt
                        {
                            if let Expression::Function(params, body) = expr.as_ref() {
                                (name.clone(), params.clone(), body.clone())
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        };

                    let capture = Rc::new(RefCell::new(Environment::with_parent(
                        HashMap::new(),
                        Rc::new(RefCell::new(parent_env.clone())),
                    )));
                    let lox_function = LoxFunction {
                        name: method_name.clone(),
                        params: params.clone(),
                        body: body.clone(),
                        closure: capture,
                        is_initializer: method_name == "init",
                    };
                    class_methods.insert(method_name, lox_function);
                }
                // Create LoxClass and store in environment
                let lox_class = Rc::new(LoxClass::new(class_name.clone(), class_methods, None));
                self.environment
                    .borrow_mut()
                    .define(class_name, DataTypes::Class(lox_class));
                None
            }
            Statment::TestStatement(_, _) => None,
        }
    }

    fn visit_expr(&mut self, e: &Expression) -> DataTypes {
        match e {
            Expression::Binary(left, op, right) => self.visit_binary(left, op, right),
            Expression::Unary(op, right) => self.visit_unary(op, right),
            Expression::Literal(token) => self.visit_literal(token),
            Expression::Grouping(exp) => self.visit_grouping(exp),
            Expression::Assign(name, value) => self.visit_assign(name, value),
            Expression::Call(name, args) => self.visit_call(name, args),
            Expression::Array(elements) => self.visit_array(elements),
            Expression::Index(object, index) => self.visit_index(object, index),
            Expression::IndexSet(object, index, value) => self.visit_index_set(object, index, value),
            Expression::Object(fields) => self.visit_object(fields),
            Expression::Get(object, name) => self.visit_get(object, name),
            Expression::Set(object, name, value) => self.visit_set(object, name, value),
            Expression::This => self.visit_this(),
            Expression::Super(method) => self.visit_super(method),
            Expression::SuperCall(method, args) => self.visit_super_call(method, args),
            Expression::Function(params, body) => self.visit_function(params, body),
            Expression::Return(value) => self.visit_return(value),
        }
    }
}

impl ExpressionVisitor<DataTypes> for CheckedInterpret {
    fn visit_binary(
        &mut self,
        left: &Expression,
        operator: &Token,
        right: &Expression,
    ) -> DataTypes {
        let left_val = self.visit_expr(left);
        let right_val = self.visit_expr(right);
        match operator.token_type {
            TokenType::MINUS => bin_expr::<f32>(left_val, right_val, |a, b| a - b),
            TokenType::PLUS => match (&left_val, &right_val) {
                (DataTypes::Number(a), DataTypes::Number(b)) => (a + b).into(),
                (DataTypes::Str(a), DataTypes::Str(b)) => DataTypes::Str(format!("{}{}", a, b)),
                (DataTypes::Str(a), DataTypes::Number(b)) => DataTypes::Str(format!("{}{}", a, b)),
                (DataTypes::Number(a), DataTypes::Str(b)) => DataTypes::Str(format!("{}{}", a, b)),
                (DataTypes::Array(a), DataTypes::Array(b)) => {
                    let mut combined = a.clone();
                    combined.extend(b.clone());
                    DataTypes::Array(combined)
                }
                _ => DataTypes::Number(0.0),
            },
            TokenType::SLASH => bin_expr::<f32>(left_val, right_val, |a, b| a / b),
            TokenType::STAR => bin_expr::<f32>(left_val, right_val, |a, b| a * b),
            TokenType::EqualEqual => {
                let l: DataTypes = left_val.clone();
                let r: DataTypes = right_val.clone();
                (l == r).into()
            }
            TokenType::BangEqual => {
                let l: DataTypes = left_val.clone();
                let r: DataTypes = right_val.clone();
                (l != r).into()
            }
            TokenType::GREATER => bin_expr::<f32>(left_val, right_val, |a, b| (a > b).into()),
            TokenType::GreaterEqual => bin_expr::<f32>(left_val, right_val, |a, b| (a >= b).into()),
            TokenType::LESS => bin_expr::<f32>(left_val, right_val, |a, b| (a < b).into()),
            TokenType::LessEqual => bin_expr::<f32>(left_val, right_val, |a, b| (a <= b).into()),
            TokenType::AND => DataTypes::Bool(bool::from(left_val) && bool::from(right_val)),
            TokenType::OR => DataTypes::Bool(bool::from(left_val) || bool::from(right_val)),
            _ => unimplemented!("Binary operator not implemented"),
        }
    }

    fn visit_unary(&mut self, operator: &Token, right: &Expression) -> DataTypes {
        let right_val = self.visit_expr(right);
        match operator.token_type {
            TokenType::MINUS => match right_val {
                DataTypes::Number(n) => (-n).into(),
                _ => unimplemented!("Unary minus only for numbers"),
            },
            TokenType::BANG => match right_val {
                DataTypes::Bool(b) => (!b).into(),
                DataTypes::Number(n) => (n == 0.0).into(),
                DataTypes::Str(s) => s.is_empty().into(),
                _ => true.into(),
            },
            _ => unimplemented!("Unary operator not implemented"),
        }
    }

    fn visit_literal(&mut self, token: &Token) -> DataTypes {
        match token.token_type {
            TokenType::NUMBER => token.literal.clone().into(),
            TokenType::STRING => token.literal.clone().into(),
            TokenType::TRUE => DataTypes::Bool(true),
            TokenType::FALSE => DataTypes::Bool(false),
            TokenType::NIL => DataTypes::Number(0.0),
            TokenType::IDENTIFIER => {
                let mut env = self.environment.borrow_mut();
                match env.check_use(&token.text, "use".to_string()) {
                    Ok(_) => env
                        .get(&token.text)
                        .unwrap_or_else(|| DataTypes::Str(format!("undefined:{}", token.text))),
                    Err(e) => {
                        let msg = format!("Borrow check error: {e}");
                        eprintln!("RECORDING ERROR: {msg}");
                        env.record_error(msg.clone());
                        env.get(&token.text)
                            .unwrap_or_else(|| DataTypes::Str(format!("undefined:{}", token.text)))
                    }
                }
            }
            _ => unimplemented!(),
        }
    }

    fn visit_grouping(&mut self, expression: &Expression) -> DataTypes {
        self.visit_expr(expression)
    }

    fn visit_assign(&mut self, name: &str, value: &Expression) -> DataTypes {
        let val = self.visit_expr(value);
        let mut env = self.environment.borrow_mut();
        match env.assign(name.to_string(), val.clone()) {
            Ok(_) => val,
            Err(e) => {
                let msg = format!("Borrow check error on assign: {e}");
                eprintln!("{msg}");
                env.record_error(msg);
                val
            }
        }
    }

    fn visit_call(&mut self, name: &str, args: &[Expression]) -> DataTypes {
        let arg_vals: Vec<DataTypes> = args.iter().map(|a| self.visit_expr(a)).collect();

        // Check for method call on instance (first arg is instance, method name is 'name')
        if !arg_vals.is_empty() {
            if let DataTypes::Instance(instance) = &arg_vals[0] {
                if let Some(DataTypes::Function {
                    params,
                    body,
                    capture,
                }) = instance.get(name)
                {
                    let mut new_env =
                        CheckedEnvironment::new(self.environment.borrow().borrow_checker.clone());
                    new_env.vars = capture.borrow().vars.clone();
                    for (i, param) in params.iter().enumerate() {
                        if i < arg_vals.len() {
                            let _ = new_env.declare(param.clone(), arg_vals[i].clone());
                        }
                    }
                    let mut interp = CheckedInterpret {
                        environment: Rc::new(RefCell::new(new_env)),
                        check_borrows: self.check_borrows,
                    };
                    return interp
                        .execute_with_borrows(&body)
                        .unwrap_or(DataTypes::Number(0.0));
                }
            }
        }

        let env = self.environment.borrow_mut();
        if let Some(func) = env.get(name) {
            match func {
                DataTypes::Class(class) => {
                    // Create instance
                    let instance = DataTypes::Instance(Rc::new(LoxInstance::new(class.clone())));
                    // If class has init method, call it
                    if let Some(init_method) = class.find_method("init") {
                        let mut new_env = env.clone();
                        new_env.push_scope();
                        for (i, param) in init_method.params.iter().enumerate() {
                            if i < arg_vals.len() {
                                let _ = new_env.declare(param.clone(), arg_vals[i].clone());
                            }
                        }
                        drop(env);
                        let borrow_checker = self.environment.borrow().borrow_checker.clone();
                        let mut interp = CheckedInterpret {
                            environment: Rc::new(RefCell::new(CheckedEnvironment::new(
                                borrow_checker,
                            ))),
                            check_borrows: self.check_borrows,
                        };
                        // Copy vars from captured environment
                        interp.environment.borrow_mut().vars =
                            init_method.closure.borrow().vars.clone();
                        interp
                            .environment
                            .borrow_mut()
                            .vars
                            .extend(new_env.vars.clone());
                        interp
                            .execute_with_borrows(&init_method.body)
                            .unwrap_or(DataTypes::Number(0.0));
                        return instance;
                    }
                    instance
                }
                DataTypes::Function {
                    params,
                    body,
                    capture,
                } => {
                    if params.len() != arg_vals.len() {
                        eprintln!(
                            "Argument count mismatch: expected {}, got {}",
                            params.len(),
                            arg_vals.len()
                        );
                        return DataTypes::Number(0.0);
                    }
                    let mut new_env = env.clone();
                    new_env.push_scope();
                    for (i, param) in params.iter().enumerate() {
                        let _ = new_env.declare(param.clone(), arg_vals[i].clone());
                    }
                    drop(env);
                    let borrow_checker = self.environment.borrow().borrow_checker.clone();
                    let mut interp = CheckedInterpret {
                        environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
                        check_borrows: self.check_borrows,
                    };
                    // Copy vars from captured environment
                    interp.environment.borrow_mut().vars = capture.borrow().vars.clone();
                    interp
                        .environment
                        .borrow_mut()
                        .vars
                        .extend(new_env.vars.clone());
                    let result = interp.execute_with_borrows(&body);
                    let child_errors = interp.environment.borrow().errors.clone();
                    if !child_errors.is_empty() {
                        self.environment.borrow_mut().errors.extend(child_errors);
                    }
                    let explicit_ret = interp.environment.borrow_mut().return_val.take();
                    explicit_ret.or(result).unwrap_or(DataTypes::Number(0.0))
                }
                _ => {
                    let msg = format!("'{name}' is not a function or class");
                    eprintln!("{msg}");
                    self.environment.borrow_mut().record_error(msg);
                    DataTypes::Number(0.0)
                }
            }
        } else {
            drop(env);
            if let Some(val) = Interpret::default().eval_builtin_call(name, args, &arg_vals) {
                return val;
            }
            let msg = format!("Undefined function or class '{name}'");
            eprintln!("{msg}");
            self.environment.borrow_mut().record_error(msg);
            DataTypes::Number(0.0)
        }
    }

    fn visit_array(&mut self, elements: &[Expression]) -> DataTypes {
        let vals: Vec<DataTypes> = elements.iter().map(|e| self.visit_expr(e)).collect();
        DataTypes::Array(vals)
    }

    fn visit_index(&mut self, object: &Expression, index: &Expression) -> DataTypes {
        let obj = self.visit_expr(object);
        let idx = self.visit_expr(index);
        match (obj, idx) {
            (DataTypes::Array(arr), DataTypes::Number(i)) => {
                let i = i as usize;
                if i < arr.len() {
                    arr[i].clone()
                } else {
                    DataTypes::Number(0.0)
                }
            }
            (DataTypes::Object(obj), DataTypes::Str(key)) => {
                obj.get(&key).cloned().unwrap_or(DataTypes::Number(0.0))
            }
            _ => DataTypes::Number(0.0),
        }
    }

    fn visit_index_set(
        &mut self,
        _object: &Expression,
        _index: &Expression,
        value: &Expression,
    ) -> DataTypes {
        self.visit_expr(value)
    }

    fn visit_object(&mut self, fields: &[(String, Expression)]) -> DataTypes {
        let mut map = std::collections::HashMap::new();
        for (key, expr) in fields {
            map.insert(key.clone(), self.visit_expr(expr));
        }
        DataTypes::Object(map)
    }

    fn visit_get(&mut self, object: &Expression, name: &str) -> DataTypes {
        let obj = self.visit_expr(object);
        match obj {
            DataTypes::Object(map) => map.get(name).cloned().unwrap_or(DataTypes::Number(0.0)),
            DataTypes::Instance(instance) => instance
                .get(name)
                .unwrap_or(DataTypes::Str("Property not found".to_string())),
            _ => DataTypes::Number(0.0),
        }
    }

    fn visit_function(&mut self, params: &[String], body: &Statment) -> DataTypes {
        // Convert CheckedEnvironment to Environment for capture
        let capture_env = Environment::new(self.environment.borrow().vars.clone());
        DataTypes::Function {
            params: params.to_vec(),
            body: Box::new(body.clone()),
            capture: Rc::new(RefCell::new(capture_env)),
        }
    }

    fn visit_return(&mut self, value: &Option<Box<Expression>>) -> DataTypes {
        let val = if let Some(expr) = value {
            self.visit_expr(expr)
        } else {
            DataTypes::Number(0.0)
        };
        self.environment.borrow_mut().return_val = Some(val.clone());
        val
    }

    fn visit_set(&mut self, object: &Expression, name: &str, value: &Expression) -> DataTypes {
        let obj = self.visit_expr(object);
        let val = self.visit_expr(value);
        match obj {
            DataTypes::Object(mut map) => {
                map.insert(name.to_string(), val.clone());
                val
            }
            _ => DataTypes::Str("Not an object".to_string()),
        }
    }

    fn visit_this(&mut self) -> DataTypes {
        DataTypes::Str("this not bound".to_string())
    }
    fn visit_super(&mut self, method: &str) -> DataTypes {
        DataTypes::Str(format!("super.{} not bound", method))
    }

    fn visit_super_call(&mut self, method: &str, args: &[Expression]) -> DataTypes {
        DataTypes::Str(format!("super.{}({}) not bound", method, args.len()))
    }
}
impl ExpressionVisitor<DataTypes> for Interpret {
    fn visit_binary(
        &mut self,
        left: &Expression,
        operator: &Token,
        right: &Expression,
    ) -> DataTypes {
        let left_val = self.visit_expr(left);
        let right_val = self.visit_expr(right);
        match operator.token_type {
            TokenType::MINUS => bin_expr::<f32>(left_val, right_val, |a, b| a - b),
            TokenType::PLUS => match (&left_val, &right_val) {
                (DataTypes::Number(a), DataTypes::Number(b)) => (a + b).into(),
                (DataTypes::Str(a), DataTypes::Str(b)) => DataTypes::Str(format!("{}{}", a, b)),
                (DataTypes::Str(a), DataTypes::Number(b)) => DataTypes::Str(format!("{}{}", a, b)),
                (DataTypes::Number(a), DataTypes::Str(b)) => DataTypes::Str(format!("{}{}", a, b)),
                (DataTypes::Array(a), DataTypes::Array(b)) => {
                    let mut combined = a.clone();
                    combined.extend(b.clone());
                    DataTypes::Array(combined)
                }
                _ => DataTypes::Number(0.0),
            },
            TokenType::SLASH => bin_expr::<f32>(left_val, right_val, |a, b| a / b),
            TokenType::STAR => bin_expr::<f32>(left_val, right_val, |a, b| a * b),
            TokenType::EqualEqual => {
                let l: DataTypes = left_val.clone();
                let r: DataTypes = right_val.clone();
                match (l, r) {
                    (DataTypes::Number(a), DataTypes::Number(b)) => (a == b).into(),
                    (DataTypes::Bool(a), DataTypes::Bool(b)) => (a == b).into(),
                    (DataTypes::Str(a), DataTypes::Str(b)) => (a == b).into(),
                    _ => false.into(),
                }
            }
            TokenType::BangEqual => {
                let l: DataTypes = left_val.clone();
                let r: DataTypes = right_val.clone();
                match (l, r) {
                    (DataTypes::Number(a), DataTypes::Number(b)) => (a != b).into(),
                    (DataTypes::Bool(a), DataTypes::Bool(b)) => (a != b).into(),
                    (DataTypes::Str(a), DataTypes::Str(b)) => (a != b).into(),
                    _ => true.into(),
                }
            }
            TokenType::LessEqual => {
                let l: f32 = left_val.into();
                let r: f32 = right_val.into();
                (l <= r).into()
            }
            TokenType::LESS => {
                let l: f32 = left_val.into();
                let r: f32 = right_val.into();
                (l < r).into()
            }
            TokenType::GreaterEqual => {
                let l: f32 = left_val.into();
                let r: f32 = right_val.into();
                (l >= r).into()
            }
            TokenType::GREATER => {
                let l: f32 = left_val.into();
                let r: f32 = right_val.into();
                (l > r).into()
            }
            TokenType::AND => DataTypes::Bool(bool::from(left_val) && bool::from(right_val)),
            TokenType::OR => DataTypes::Bool(bool::from(left_val) || bool::from(right_val)),
            _ => unimplemented!(),
        }
    }

    fn visit_unary(&mut self, operator: &Token, right: &Expression) -> DataTypes {
        let right_val = self.visit_expr(right);
        match operator.token_type {
            TokenType::MINUS => { -f32::from(right_val) }.into(),
            TokenType::BANG => { !bool::from(right_val) }.into(),
            _ => unimplemented!(),
        }
    }

    fn visit_literal(&mut self, token: &Token) -> DataTypes {
        match token.token_type {
            TokenType::NUMBER => token.literal.clone().into(),
            TokenType::STRING => token.literal.clone().into(),
            TokenType::TRUE => DataTypes::Bool(true),
            TokenType::FALSE => DataTypes::Bool(false),
            TokenType::NIL => DataTypes::Number(0.0),
            TokenType::IDENTIFIER => self
                .environment
                .borrow()
                .get(&token.text)
                .unwrap_or_else(|| DataTypes::Str(format!("undefined:{}", token.text))),
            _ => unimplemented!(),
        }
    }

    fn visit_grouping(&mut self, expression: &Expression) -> DataTypes {
        self.visit_expr(expression)
    }

    fn visit_assign(&mut self, name: &str, value: &Expression) -> DataTypes {
        let val = self.visit_expr(value);
        self.environment
            .borrow_mut()
            .set(name.to_string(), val.clone());
        val
    }
    fn visit_call(&mut self, name: &str, args: &[Expression]) -> DataTypes {
        // Evaluate all arguments first
        let arg_vals: Vec<DataTypes> = args.iter().map(|a| self.visit_expr(a)).collect();

        // Check for class instantiation
        let class = match self.environment.borrow().get(name) {
            Some(DataTypes::Class(c)) => Some(c.clone()),
            _ => None,
        };
        if let Some(class) = class {
            // Create instance
            let instance = DataTypes::Instance(Rc::new(LoxInstance::new(class.clone())));
            // If class has init method, call it
            if let Some(init_method) = class.find_method("init") {
                let mut new_env =
                    Environment::with_parent(HashMap::new(), init_method.closure.clone());
                for (i, param) in init_method.params.iter().enumerate() {
                    if i < arg_vals.len() {
                        new_env.define(param.clone(), arg_vals[i].clone());
                    }
                }
                let old_env = self.environment.clone();
                self.environment = Rc::new(RefCell::new(new_env));
                let result = self.execute(&init_method.body);
                self.environment = old_env;
                return result.unwrap_or(DataTypes::Number(0.0));
            }
            return instance;
        }

        // Check for method call on instance (first arg is instance, method name is 'name')
        if !arg_vals.is_empty() {
            if let DataTypes::Instance(instance) = &arg_vals[0] {
                // Look up method on instance
                if let Some(DataTypes::Function {
                    params,
                    body,
                    capture,
                }) = instance.get(name)
                {
                    // Bind 'this' to the instance
                    let mut new_env = Environment::with_parent(HashMap::new(), capture.clone());
                    // First param is 'this' (or implicit receiver)
                    // The parser already prepends the instance as first arg
                    for (i, param) in params.iter().enumerate() {
                        if i < arg_vals.len() {
                            new_env.define(param.clone(), arg_vals[i].clone());
                        }
                    }
                    let old_env = self.environment.clone();
                    self.environment = Rc::new(RefCell::new(new_env));
                    let result = self.execute(&body);
                    self.environment = old_env;
                    return result.unwrap_or(DataTypes::Number(0.0));
                }
            }
        }
        // Check for user-defined function in environment
        let func_data = self.environment.borrow().get(name);
        if let Some(DataTypes::Function {
            params,
            body,
            capture,
        }) = func_data
        {
            // Create new environment with captured vars
            let mut new_env = Environment::with_parent(HashMap::new(), capture.clone());
            // Bind parameters
            for (i, param) in params.iter().enumerate() {
                if i < arg_vals.len() {
                    new_env.define(param.clone(), arg_vals[i].clone());
                }
            }
            let old_env = self.environment.clone();
            self.environment = Rc::new(RefCell::new(new_env));
            let result = self.execute(&body);
            let explicit_ret = self.environment.borrow_mut().return_val.take();
            self.environment = old_env;
            if let Some(ret) = explicit_ret {
                return ret;
            }
            return result.unwrap_or(DataTypes::Number(0.0));
        }

        if let Some(val) = self.eval_builtin_call(name, args, &arg_vals) {
            return val;
        }
        DataTypes::Str(format!("Unknown function: {}", name))
    }

    fn visit_array(&mut self, elements: &[Expression]) -> DataTypes {
        DataTypes::Array(elements.iter().map(|e| self.visit_expr(e)).collect())
    }

    fn visit_index(&mut self, object: &Expression, index: &Expression) -> DataTypes {
        let obj = self.visit_expr(object);
        let idx = self.visit_expr(index);
        match (&obj, &idx) {
            (DataTypes::Array(arr), DataTypes::Number(i)) => {
                let i = *i as usize;
                if i < arr.len() {
                    arr[i].clone()
                } else {
                    DataTypes::Str(format!("Index out of bounds: {}", i))
                }
            }
            (DataTypes::Object(map), DataTypes::Str(key)) => {
                map.get(key).cloned().unwrap_or(DataTypes::Number(0.0))
            }
            _ => DataTypes::Str("Invalid index operation".to_string()),
        }
    }

    fn visit_index_set(
        &mut self,
        object: &Expression,
        index: &Expression,
        value: &Expression,
    ) -> DataTypes {
        let val = self.visit_expr(value);
        let idx = self.visit_expr(index);
        if let Expression::Literal(tok) = object {
            if tok.token_type == TokenType::IDENTIFIER {
                let target = self.environment.borrow().get(&tok.text);
                if let Some(target) = target {
                    match (target, &idx) {
                        (DataTypes::Array(mut arr), DataTypes::Number(i)) => {
                            let i = *i as usize;
                            if i < arr.len() {
                                arr[i] = val.clone();
                                self.environment
                                    .borrow_mut()
                                    .set(tok.text.clone(), DataTypes::Array(arr));
                                return val;
                            }
                        }
                        (DataTypes::Object(mut map), DataTypes::Str(k)) => {
                            map.insert(k.clone(), val.clone());
                            self.environment
                                .borrow_mut()
                                .set(tok.text.clone(), DataTypes::Object(map));
                            return val;
                        }
                        _ => {}
                    }
                }
            }
        }
        val
    }

    fn visit_object(&mut self, fields: &[(String, Expression)]) -> DataTypes {
        let mut map = std::collections::HashMap::new();
        for (key, expr) in fields {
            map.insert(key.clone(), self.visit_expr(expr));
        }
        DataTypes::Object(map)
    }

    fn visit_get(&mut self, object: &Expression, name: &str) -> DataTypes {
        let obj = self.visit_expr(object);
        match obj {
            DataTypes::Object(map) => map.get(name).cloned().unwrap_or(DataTypes::Number(0.0)),
            DataTypes::Instance(instance) => instance
                .get(name)
                .unwrap_or(DataTypes::Str("Property not found".to_string())),
            _ => DataTypes::Str("Not an object".to_string()),
        }
    }
    fn visit_function(&mut self, params: &[String], body: &Statment) -> DataTypes {
        let parent_env = self.environment.clone();
        let capture = Rc::new(RefCell::new(Environment::with_parent(
            HashMap::new(),
            parent_env,
        )));
        DataTypes::Function {
            params: params.to_vec(),
            body: Box::new(body.clone()),
            capture,
        }
    }

    fn visit_return(&mut self, value: &Option<Box<Expression>>) -> DataTypes {
        let val = match value {
            Some(expr) => self.visit_expr(expr),
            None => DataTypes::Number(0.0),
        };
        self.environment.borrow_mut().return_val = Some(val.clone());
        val
    }

    fn visit_set(&mut self, object: &Expression, name: &str, value: &Expression) -> DataTypes {
        let obj = self.visit_expr(object);
        let val = self.visit_expr(value);
        match obj {
            DataTypes::Object(mut map) => {
                map.insert(name.to_string(), val.clone());
                val
            }
            _ => DataTypes::Str("Not an object".to_string()),
        }
    }

    fn visit_this(&mut self) -> DataTypes {
        DataTypes::Str("this not bound".to_string())
    }

    fn visit_super(&mut self, method: &str) -> DataTypes {
        DataTypes::Str(format!("super.{} not bound", method))
    }

    fn visit_super_call(&mut self, method: &str, args: &[Expression]) -> DataTypes {
        // Evaluate arguments
        let arg_vals: Vec<DataTypes> = args.iter().map(|a| self.visit_expr(a)).collect();

        // Get 'this' from environment (should be the current instance)
        let this_instance = match self.environment.borrow().get("this") {
            Some(DataTypes::Instance(instance)) => instance,
            _ => return DataTypes::Str("super call: 'this' not bound".to_string()),
        };

        // Get superclass from the instance's class
        if let Some(superclass) = &this_instance.class.superclass {
            if let Some(super_method) = superclass.find_method(method) {
                // Bind 'this' to the current instance
                let mut new_env =
                    Environment::with_parent(HashMap::new(), super_method.closure.clone());
                for (i, param) in super_method.params.iter().enumerate() {
                    if i < arg_vals.len() {
                        new_env.define(param.clone(), arg_vals[i].clone());
                    }
                }
                let old_env = self.environment.clone();
                self.environment = Rc::new(RefCell::new(new_env));
                let result = self.execute(&super_method.body);
                self.environment = old_env;
                return result.unwrap_or(DataTypes::Number(0.0));
            }
        }

        DataTypes::Str(format!("super method '{}' not found", method))
    }
}

impl Interpret {
    pub fn eval_builtin_call(
        &mut self,
        name: &str,
        args: &[Expression],
        arg_vals: &[DataTypes],
    ) -> Option<DataTypes> {
        let res = self.eval_builtin_inner(name, args, arg_vals);
        if let DataTypes::Str(ref s) = res {
            if s.starts_with("__PSUDO_UNKNOWN_FN__:") {
                return None;
            }
        }
        Some(res)
    }

    fn eval_builtin_inner(
        &mut self,
        name: &str,
        args: &[Expression],
        arg_vals: &[DataTypes],
    ) -> DataTypes {
        match name {
            "readFile" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("readFile expects 1 argument".to_string());
                }
                let path = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("readFile expects string argument".to_string()),
                };
                match crate::fs::read_file(&path) {
                    Ok(content) => DataTypes::Str(content),
                    Err(e) => DataTypes::Str(format!("Error: {}", e)),
                }
            }
            "writeFile" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("writeFile expects 2 arguments".to_string());
                }
                let path = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("writeFile expects string path".to_string()),
                };
                let content = match &arg_vals[1] {
                    DataTypes::Str(s) => s.clone(),
                    DataTypes::Number(n) => n.to_string(),
                    DataTypes::Bool(b) => b.to_string(),
                    DataTypes::Array(arr) => format!(
                        "[{}]",
                        arr.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    DataTypes::Object(obj) => format!(
                        "{{{}}}",
                        obj.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    DataTypes::Function { params, .. } => format!("<fn {}>", params.join(", ")),
                    DataTypes::Class(class) => format!("<class {}>", class.name),
                    DataTypes::Instance(instance) => {
                        format!("<instance of {}>", instance.class.name)
                    }
                    DataTypes::Result(r) => match r {
                        Ok(v) => format!("Ok({})", v),
                        Err(e) => format!("Err({})", e),
                    },
                };
                match crate::fs::write_file(&path, &content) {
                    Ok(_) => DataTypes::Number(0.0), // nil equivalent
                    Err(e) => DataTypes::Str(format!("Error: {}", e)),
                }
            }
            "listDir" => {
                let path = if arg_vals.is_empty() {
                    ".".to_string()
                } else {
                    match &arg_vals[0] {
                        DataTypes::Str(s) => s.clone(),
                        _ => return DataTypes::Str("listDir expects string argument".to_string()),
                    }
                };
                match crate::fs::list_dir(&path) {
                    Ok(entries) => {
                        let joined = entries.join("\n");
                        DataTypes::Str(joined)
                    }
                    Err(e) => DataTypes::Str(format!("Error: {}", e)),
                }
            }
            "exists" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("exists expects 1 argument".to_string());
                }
                let path = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("exists expects string argument".to_string()),
                };
                DataTypes::Bool(crate::fs::exists(&path))
            }
            "isFile" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("isFile expects 1 argument".to_string());
                }
                let path = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("isFile expects string argument".to_string()),
                };
                DataTypes::Bool(crate::fs::is_file(&path))
            }
            "isDir" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("isDir expects 1 argument".to_string());
                }
                let path = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("isDir expects string argument".to_string()),
                };
                DataTypes::Bool(crate::fs::is_dir(&path))
            }
            "mkdir" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("mkdir expects 1 argument".to_string());
                }
                let path = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("mkdir expects string argument".to_string()),
                };
                match crate::fs::mkdir(&path) {
                    Ok(_) => DataTypes::Number(0.0),
                    Err(e) => DataTypes::Str(format!("Error: {}", e)),
                }
            }
            "len" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("len expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Array(arr) => DataTypes::Number(arr.len() as f32),
                    DataTypes::Str(s) => DataTypes::Number(s.len() as f32),
                    DataTypes::Object(obj) => DataTypes::Number(obj.len() as f32),
                    _ => DataTypes::Str("len expects array, string, or object".to_string()),
                }
            }
            "push" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("push expects 2 arguments".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.push(arg_vals[1].clone());
                        if let Expression::Literal(tok) = &args[0] {
                            if tok.token_type == TokenType::IDENTIFIER {
                                self.environment
                                    .borrow_mut()
                                    .set(tok.text.clone(), DataTypes::Array(new_arr.clone()));
                            }
                        }
                        DataTypes::Array(new_arr)
                    }
                    _ => DataTypes::Str("push expects array as first argument".to_string()),
                }
            }
            "pop" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("pop expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.pop();
                        DataTypes::Array(new_arr)
                    }
                    _ => DataTypes::Str("pop expects array".to_string()),
                }
            }
            "slice" => {
                if arg_vals.len() < 2 || arg_vals.len() > 3 {
                    return DataTypes::Str("slice expects 2 or 3 arguments".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Array(arr) => {
                        let start_num: f32 = arg_vals[1].clone().into();
                        let start = start_num as i32;
                        let end = if arg_vals.len() == 3 {
                            let end_num: f32 = arg_vals[2].clone().into();
                            end_num as i32
                        } else {
                            arr.len() as i32
                        };
                        let start = if start < 0 {
                            (arr.len() as i32 + start).max(0) as usize
                        } else {
                            start as usize
                        };
                        let end = if end < 0 {
                            (arr.len() as i32 + end).max(0) as usize
                        } else {
                            end as usize
                        };
                        let end = end.min(arr.len());
                        if start <= end && start < arr.len() {
                            DataTypes::Array(arr[start..end].to_vec())
                        } else {
                            DataTypes::Array(vec![])
                        }
                    }
                    _ => DataTypes::Str("slice expects array".to_string()),
                }
            }
            "keys" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("keys expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Object(obj) => {
                        let keys: Vec<DataTypes> =
                            obj.keys().map(|k| DataTypes::Str(k.clone())).collect();
                        DataTypes::Array(keys)
                    }
                    _ => DataTypes::Str("keys expects object".to_string()),
                }
            }
            "values" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("values expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Object(obj) => {
                        let vals: Vec<DataTypes> = obj.values().cloned().collect();
                        DataTypes::Array(vals)
                    }
                    _ => DataTypes::Str("values expects object".to_string()),
                }
            }
            "has" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("has expects 2 arguments".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Object(obj) => {
                        let key = match &arg_vals[1] {
                            DataTypes::Str(s) => s.clone(),
                            _ => return DataTypes::Str("has expects string key".to_string()),
                        };
                        DataTypes::Bool(obj.contains_key(&key))
                    }
                    _ => DataTypes::Str("has expects object".to_string()),
                }
            }
            "ok" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("ok expects 1 argument".to_string());
                }
                DataTypes::Result(Ok(Box::new(arg_vals[0].clone())))
            }
            "err" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("err expects 1 argument".to_string());
                }
                DataTypes::Result(Err(Box::new(arg_vals[0].clone())))
            }
            "isError" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("isError expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Result(r) => DataTypes::Bool(r.is_err()),
                    _ => DataTypes::Bool(false),
                }
            }
            "isOk" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("isOk expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Result(r) => DataTypes::Bool(r.is_ok()),
                    _ => DataTypes::Bool(false),
                }
            }
            "unwrap" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("unwrap expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Result(r) => match r {
                        Ok(v) => *v.clone(),
                        Err(e) => DataTypes::Str(format!("unwrap on Err: {}", e)),
                    },
                    _ => DataTypes::Str("unwrap expects Result".to_string()),
                }
            }
            "assert" => {
                if arg_vals.is_empty() || arg_vals.len() > 2 {
                    return DataTypes::Str("assert expects 1 or 2 arguments".to_string());
                }
                let cond: bool = arg_vals[0].clone().into();
                let msg = if arg_vals.len() == 2 {
                    match &arg_vals[1] {
                        DataTypes::Str(s) => s.clone(),
                        _ => "assertion failed".to_string(),
                    }
                } else {
                    "assertion failed".to_string()
                };
                if cond {
                    DataTypes::Result(Ok(Box::new(DataTypes::Bool(true))))
                } else {
                    self.environment
                        .borrow_mut()
                        .assertion_failures
                        .push(msg.clone());
                    DataTypes::Result(Err(Box::new(DataTypes::Str(msg))))
                }
            }
            "assert_eq" => {
                if arg_vals.len() < 2 || arg_vals.len() > 3 {
                    return DataTypes::Str("assert_eq expects 2 or 3 arguments".to_string());
                }
                let left = &arg_vals[0];
                let right = &arg_vals[1];
                let msg = if arg_vals.len() == 3 {
                    match &arg_vals[2] {
                        DataTypes::Str(s) => s.clone(),
                        _ => format!("assertion failed: {} != {}", left, right),
                    }
                } else {
                    format!("assertion failed: {} != {}", left, right)
                };
                if left == right {
                    DataTypes::Result(Ok(Box::new(DataTypes::Bool(true))))
                } else {
                    self.environment
                        .borrow_mut()
                        .assertion_failures
                        .push(msg.clone());
                    DataTypes::Result(Err(Box::new(DataTypes::Str(msg))))
                }
            }
            "assert_ne" => {
                if arg_vals.len() < 2 || arg_vals.len() > 3 {
                    return DataTypes::Str("assert_ne expects 2 or 3 arguments".to_string());
                }
                let left = &arg_vals[0];
                let right = &arg_vals[1];
                let msg = if arg_vals.len() == 3 {
                    match &arg_vals[2] {
                        DataTypes::Str(s) => s.clone(),
                        _ => format!("assertion failed: {} == {}", left, right),
                    }
                } else {
                    format!("assertion failed: {} == {}", left, right)
                };
                if left != right {
                    DataTypes::Result(Ok(Box::new(DataTypes::Bool(true))))
                } else {
                    self.environment
                        .borrow_mut()
                        .assertion_failures
                        .push(msg.clone());
                    DataTypes::Result(Err(Box::new(DataTypes::Str(msg))))
                }
            }
            "sqrt" => {
                let n: f32 = arg_vals[0].clone().into();
                DataTypes::Number(n.sqrt())
            }
            "pow" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("pow expects 2 arguments".to_string());
                }
                let base: f32 = arg_vals[0].clone().into();
                let exp: f32 = arg_vals[1].clone().into();
                DataTypes::Number(base.powf(exp))
            }
            "abs" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("abs expects 1 argument".to_string());
                }
                let n: f32 = arg_vals[0].clone().into();
                DataTypes::Number(n.abs())
            }
            "min" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("min expects 2 arguments".to_string());
                }
                let a: f32 = arg_vals[0].clone().into();
                let b: f32 = arg_vals[1].clone().into();
                DataTypes::Number(a.min(b))
            }
            "max" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("max expects 2 arguments".to_string());
                }
                let a: f32 = arg_vals[0].clone().into();
                let b: f32 = arg_vals[1].clone().into();
                DataTypes::Number(a.max(b))
            }
            "round" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("round expects 1 argument".to_string());
                }
                let n: f32 = arg_vals[0].clone().into();
                DataTypes::Number(n.round())
            }
            "floor" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("floor expects 1 argument".to_string());
                }
                let n: f32 = arg_vals[0].clone().into();
                DataTypes::Number(n.floor())
            }
            "ceil" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("ceil expects 1 argument".to_string());
                }
                let n: f32 = arg_vals[0].clone().into();
                DataTypes::Number(n.ceil())
            }
            "time_now_sec" | "time_now_secs" | "clock" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f32())
                    .unwrap_or(0.0);
                DataTypes::Number(now)
            }
            "time_micros" => {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                let start = START.get_or_init(std::time::Instant::now);
                DataTypes::Number(start.elapsed().as_micros() as f32)
            }
            "time_nanos" => {
                static START_NANO: std::sync::OnceLock<std::time::Instant> =
                    std::sync::OnceLock::new();
                let start = START_NANO.get_or_init(std::time::Instant::now);
                DataTypes::Number((start.elapsed().as_nanos() % 1_000_000_000) as f32)
            }
            "sleep_ms" => {
                let ms = match arg_vals.first() {
                    Some(DataTypes::Number(n)) => *n as u64,
                    _ => 0,
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                DataTypes::Bool(true)
            }
            "benchmark" => {
                let iters = match arg_vals.first() {
                    Some(DataTypes::Number(n)) => *n as u32,
                    _ => 1,
                };
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    if let Some(arg) = arg_vals.get(1) {
                        let _ = arg;
                    }
                }
                let elapsed = start.elapsed();
                let total_us = elapsed.as_micros() as f32;
                let avg_us = if iters > 0 {
                    total_us / (iters as f32)
                } else {
                    0.0
                };

                let mut map = HashMap::new();
                map.insert("iterations".to_string(), DataTypes::Number(iters as f32));
                map.insert("total_us".to_string(), DataTypes::Number(total_us));
                map.insert("avg_us".to_string(), DataTypes::Number(avg_us));
                DataTypes::Object(map)
            }
            "rate_limiter_sliding_window" => {
                let window_sec = match arg_vals.first() {
                    Some(DataTypes::Number(n)) => *n as u64,
                    _ => 60,
                };
                let max_reqs = match arg_vals.get(1) {
                    Some(DataTypes::Number(n)) => *n as usize,
                    _ => 100,
                };
                let mut map = HashMap::new();
                map.insert(
                    "__type".to_string(),
                    DataTypes::Str("sliding_window".to_string()),
                );
                map.insert(
                    "window_sec".to_string(),
                    DataTypes::Number(window_sec as f32),
                );
                map.insert("max_reqs".to_string(), DataTypes::Number(max_reqs as f32));
                DataTypes::Object(map)
            }
            "rate_limiter_token_bucket" => {
                let capacity = match arg_vals.first() {
                    Some(DataTypes::Number(n)) => *n,
                    _ => 100.0,
                };
                let refill_rate = match arg_vals.get(1) {
                    Some(DataTypes::Number(n)) => *n,
                    _ => 10.0,
                };
                let mut map = HashMap::new();
                map.insert(
                    "__type".to_string(),
                    DataTypes::Str("token_bucket".to_string()),
                );
                map.insert("capacity".to_string(), DataTypes::Number(capacity));
                map.insert("refill_rate".to_string(), DataTypes::Number(refill_rate));
                DataTypes::Object(map)
            }
            "persistent_kv" => {
                let store_name = match arg_vals.first() {
                    Some(DataTypes::Str(s)) => s.clone(),
                    _ => "default_store".to_string(),
                };
                let mut map = HashMap::new();
                map.insert(
                    "__type".to_string(),
                    DataTypes::Str("persistent_kv".to_string()),
                );
                map.insert("store_name".to_string(), DataTypes::Str(store_name));
                DataTypes::Object(map)
            }
            "consume" => {
                if arg_vals.len() >= 3 {
                    if let DataTypes::Object(obj) = &arg_vals[0] {
                        if let Some(DataTypes::Str(t)) = obj.get("__type") {
                            let client_id = match &arg_vals[1] {
                                DataTypes::Str(s) => s.clone(),
                                other => format!("{}", other),
                            };
                            let timestamp = match &arg_vals[2] {
                                DataTypes::Number(n) => *n as u64,
                                _ => 0,
                            };
                            if t == "sliding_window" {
                                let window_sec = obj
                                    .get("window_sec")
                                    .and_then(|v| match v {
                                        DataTypes::Number(n) => Some(*n as u64),
                                        _ => None,
                                    })
                                    .unwrap_or(60);
                                let max_reqs = obj
                                    .get("max_reqs")
                                    .and_then(|v| match v {
                                        DataTypes::Number(n) => Some(*n as usize),
                                        _ => None,
                                    })
                                    .unwrap_or(100);
                                let path = std::path::PathBuf::from(
                                    ".pseudo/state/sliding_window_state.json",
                                );
                                let mut store: HashMap<String, Vec<u64>> = if path.exists() {
                                    std::fs::read_to_string(&path)
                                        .ok()
                                        .and_then(|s| serde_json::from_str(&s).ok())
                                        .unwrap_or_default()
                                } else {
                                    HashMap::new()
                                };
                                let times = store.entry(client_id).or_default();
                                times.retain(|&t| t + window_sec > timestamp);
                                if times.len() < max_reqs {
                                    times.push(timestamp);
                                    if let Some(parent) = path.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    let _ = std::fs::write(
                                        &path,
                                        serde_json::to_string(&store).unwrap_or_default(),
                                    );
                                    DataTypes::Bool(true)
                                } else {
                                    DataTypes::Bool(false)
                                }
                            } else if t == "token_bucket" {
                                let capacity = obj
                                    .get("capacity")
                                    .and_then(|v| match v {
                                        DataTypes::Number(n) => Some(*n),
                                        _ => None,
                                    })
                                    .unwrap_or(100.0);
                                let refill_rate = obj
                                    .get("refill_rate")
                                    .and_then(|v| match v {
                                        DataTypes::Number(n) => Some(*n),
                                        _ => None,
                                    })
                                    .unwrap_or(10.0);
                                let path = std::path::PathBuf::from(
                                    ".pseudo/state/token_bucket_state.json",
                                );
                                let mut store: HashMap<String, (f32, u64)> = if path.exists() {
                                    std::fs::read_to_string(&path)
                                        .ok()
                                        .and_then(|s| serde_json::from_str(&s).ok())
                                        .unwrap_or_default()
                                } else {
                                    HashMap::new()
                                };
                                let (mut tokens, mut last_ts) = store
                                    .entry(client_id.clone())
                                    .or_insert((capacity, timestamp));
                                let elapsed = if timestamp > last_ts {
                                    (timestamp - last_ts) as f32
                                } else {
                                    0.0
                                };
                                tokens = (tokens + elapsed * refill_rate).min(capacity);
                                last_ts = timestamp;
                                if tokens >= 1.0 {
                                    tokens -= 1.0;
                                    store.insert(client_id, (tokens, last_ts));
                                    if let Some(parent) = path.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    let _ = std::fs::write(
                                        &path,
                                        serde_json::to_string(&store).unwrap_or_default(),
                                    );
                                    DataTypes::Bool(true)
                                } else {
                                    store.insert(client_id, (tokens, last_ts));
                                    DataTypes::Bool(false)
                                }
                            } else {
                                DataTypes::Bool(true)
                            }
                        } else {
                            DataTypes::Bool(false)
                        }
                    } else {
                        DataTypes::Bool(false)
                    }
                } else {
                    DataTypes::Bool(false)
                }
            }
            "get" => {
                if arg_vals.len() >= 2 {
                    if let DataTypes::Object(obj) = &arg_vals[0] {
                        if let Some(DataTypes::Str(store_name)) = obj.get("store_name") {
                            let key = match &arg_vals[1] {
                                DataTypes::Str(s) => s.clone(),
                                other => format!("{}", other),
                            };
                            let default_val =
                                arg_vals.get(2).cloned().unwrap_or(DataTypes::Number(0.0));
                            let path = std::path::PathBuf::from(format!(
                                ".pseudo/state/{}.json",
                                store_name
                            ));
                            if path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(map) = serde_json::from_str::<
                                        HashMap<String, serde_json::Value>,
                                    >(&content)
                                    {
                                        if let Some(val) = map.get(&key) {
                                            if let Some(n) = val.as_f64() {
                                                return DataTypes::Number(n as f32);
                                            } else if let Some(s) = val.as_str() {
                                                return DataTypes::Str(s.to_string());
                                            } else if let Some(b) = val.as_bool() {
                                                return DataTypes::Bool(b);
                                            }
                                        }
                                    }
                                }
                            }
                            default_val
                        } else {
                            DataTypes::Number(0.0)
                        }
                    } else {
                        DataTypes::Number(0.0)
                    }
                } else {
                    DataTypes::Number(0.0)
                }
            }
            "set" => {
                if arg_vals.len() >= 3 {
                    if let DataTypes::Object(obj) = &arg_vals[0] {
                        if let Some(DataTypes::Str(store_name)) = obj.get("store_name") {
                            let key = match &arg_vals[1] {
                                DataTypes::Str(s) => s.clone(),
                                other => format!("{}", other),
                            };
                            let val = &arg_vals[2];
                            let path = std::path::PathBuf::from(format!(
                                ".pseudo/state/{}.json",
                                store_name
                            ));
                            let mut map: HashMap<String, serde_json::Value> = if path.exists() {
                                std::fs::read_to_string(&path)
                                    .ok()
                                    .and_then(|s| serde_json::from_str(&s).ok())
                                    .unwrap_or_default()
                            } else {
                                HashMap::new()
                            };
                            let json_val = match val {
                                DataTypes::Number(n) => serde_json::json!(*n),
                                DataTypes::Str(s) => serde_json::json!(s),
                                DataTypes::Bool(b) => serde_json::json!(*b),
                                _ => serde_json::json!(0),
                            };
                            map.insert(key, json_val);
                            if let Some(parent) = path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(
                                &path,
                                serde_json::to_string(&map).unwrap_or_default(),
                            );
                            DataTypes::Bool(true)
                        } else {
                            DataTypes::Bool(false)
                        }
                    } else {
                        DataTypes::Bool(false)
                    }
                } else {
                    DataTypes::Bool(false)
                }
            }
            "deque_new" => DataTypes::Array(vec![]),
            "push_front" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("push_front expects 2 arguments".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Array(arr) => {
                        let mut new_arr = vec![arg_vals[1].clone()];
                        new_arr.extend(arr.clone());
                        DataTypes::Array(new_arr)
                    }
                    _ => DataTypes::Str("push_front expects array".to_string()),
                }
            }
            "pop_front" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("pop_front expects 1 argument".to_string());
                }
                match &arg_vals[0] {
                    DataTypes::Array(arr) => {
                        if arr.is_empty() {
                            DataTypes::Number(0.0)
                        } else {
                            arr[0].clone()
                        }
                    }
                    _ => DataTypes::Str("pop_front expects array".to_string()),
                }
            }
            "fmt_currency" => {
                if arg_vals.len() < 2 || arg_vals.len() > 3 {
                    return DataTypes::Str("fmt_currency expects 2 or 3 arguments".to_string());
                }
                let amt: f32 = arg_vals[0].clone().into();
                let sym = match &arg_vals[1] {
                    DataTypes::Str(s) => s.clone(),
                    _ => "$".to_string(),
                };
                let decimals = if arg_vals.len() == 3 {
                    let d: f32 = arg_vals[2].clone().into();
                    d as usize
                } else {
                    2
                };
                DataTypes::Str(format!("{}{:.*}", sym, decimals, amt))
            }
            "load_json" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("load_json expects 1 argument".to_string());
                }
                let p_str = match &arg_vals[0] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("load_json expects string path".to_string()),
                };
                let path = std::path::Path::new(".pseudo/sandbox").join(&p_str);
                let content = std::fs::read_to_string(&path)
                    .or_else(|_| std::fs::read_to_string(&p_str))
                    .unwrap_or_else(|_| "{}".to_string());
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                    json_to_datatypes(&json_val)
                } else {
                    DataTypes::Object(std::collections::HashMap::new())
                }
            }
            "expect" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("expect expects 2 arguments".to_string());
                }
                let msg = match &arg_vals[1] {
                    DataTypes::Str(s) => s.clone(),
                    _ => return DataTypes::Str("expect expects string message".to_string()),
                };
                match &arg_vals[0] {
                    DataTypes::Result(r) => match r {
                        Ok(v) => *v.clone(),
                        Err(e) => DataTypes::Str(format!("{}: {}", msg, e)),
                    },
                    _ => DataTypes::Str("expect expects Result".to_string()),
                }
            }
            "tui_metric" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("tui_metric expects 2 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("metric".to_string()));
                m.insert("label".to_string(), arg_vals[0].clone());
                m.insert("value".to_string(), arg_vals[1].clone());
                DataTypes::Object(m)
            }
            "tui_gauge" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("tui_gauge expects 2 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("gauge".to_string()));
                m.insert("label".to_string(), arg_vals[0].clone());
                m.insert("percent".to_string(), arg_vals[1].clone());
                DataTypes::Object(m)
            }
            "tui_sparkline" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("tui_sparkline expects 2 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("sparkline".to_string()));
                m.insert("label".to_string(), arg_vals[0].clone());
                m.insert("data".to_string(), arg_vals[1].clone());
                DataTypes::Object(m)
            }
            "tui_table" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("tui_table expects 2 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("table".to_string()));
                m.insert("title".to_string(), arg_vals[0].clone());
                m.insert("rows".to_string(), arg_vals[1].clone());
                DataTypes::Object(m)
            }
            "tui_log" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("tui_log expects 2 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("log".to_string()));
                m.insert("title".to_string(), arg_vals[0].clone());
                m.insert("lines".to_string(), arg_vals[1].clone());
                DataTypes::Object(m)
            }
            "tui_canvas" => {
                if arg_vals.len() != 2 {
                    return DataTypes::Str("tui_canvas expects 2 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("canvas".to_string()));
                m.insert("title".to_string(), arg_vals[0].clone());
                m.insert("matrix".to_string(), arg_vals[1].clone());
                DataTypes::Object(m)
            }
            "tui_barchart" => {
                if arg_vals.len() != 3 {
                    return DataTypes::Str("tui_barchart expects 3 arguments".to_string());
                }
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str("barchart".to_string()));
                m.insert("title".to_string(), arg_vals[0].clone());
                m.insert("labels".to_string(), arg_vals[1].clone());
                m.insert("values".to_string(), arg_vals[2].clone());
                DataTypes::Object(m)
            }
            "tui_row" | "tui_col" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str(format!("{} expects 1 argument", name));
                }
                let kind = name.strip_prefix("tui_").unwrap_or("col");
                let mut m = std::collections::HashMap::new();
                m.insert("__tui".to_string(), DataTypes::Str(kind.to_string()));
                m.insert("items".to_string(), arg_vals[0].clone());
                DataTypes::Object(m)
            }
            "tui_dashboard" | "tui_grid" | "tui_fractal" | "tui_render" => {
                let title = arg_vals
                    .first()
                    .map(|v| format!("{}", v))
                    .unwrap_or_else(|| "psudo TUI".to_string());
                println!("[tui:{}] {}", name, title);
                DataTypes::Str("__tui_rendered__".to_string())
            }
            "tokio_sleep_ms" => {
                let ms = match arg_vals.first() {
                    Some(DataTypes::Number(n)) => (*n).max(0.0) as u64,
                    _ => 0,
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                DataTypes::Bool(true)
            }
            "str" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("str expects 1 argument".to_string());
                }
                let s = match &arg_vals[0] {
                    DataTypes::Str(st) => st.clone(),
                    DataTypes::Number(n) => {
                        let rounded = (*n * 10000.0).round() / 10000.0;
                        if (rounded - rounded.round()).abs() < 1e-4 {
                            format!("{}", rounded.round() as i64)
                        } else {
                            format!("{}", rounded)
                        }
                    }
                    other => format!("{}", other),
                };
                DataTypes::Str(s)
            }
            "num" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("num expects 1 argument".to_string());
                }
                let n: f32 = match &arg_vals[0] {
                    DataTypes::Number(v) => *v,
                    DataTypes::Str(s) => s.parse().unwrap_or(0.0),
                    DataTypes::Bool(b) => if *b { 1.0 } else { 0.0 },
                    _ => 0.0,
                };
                DataTypes::Number(n)
            }
            "bool" => {
                if arg_vals.len() != 1 {
                    return DataTypes::Str("bool expects 1 argument".to_string());
                }
                DataTypes::Bool(bool::from(arg_vals[0].clone()))
            }
            _ => {
                let cfg = crate::config::load_project_config();
                for addon in cfg.addons.values() {
                    if let Some(mock) = addon.mock_returns.get(name) {
                        return DataTypes::Str(mock.clone());
                    }
                    if addon.builtins.iter().any(|b| b == name) {
                        return DataTypes::Str(format!("[addon:{}]", name));
                    }
                }
                DataTypes::Str(format!("__PSUDO_UNKNOWN_FN__:{}", name))
            }
        }
    }
    pub fn visit_expr(&mut self, e: &Expression) -> DataTypes {
        match e {
            Expression::Binary(left, op, right) => self.visit_binary(left, op, right),
            Expression::Unary(op, right) => self.visit_unary(op, right),
            Expression::Literal(token) => self.visit_literal(token),
            Expression::Grouping(exp) => self.visit_grouping(exp),
            Expression::Assign(name, value) => self.visit_assign(name, value),
            Expression::Call(name, args) => self.visit_call(name, args),
            Expression::Array(elements) => self.visit_array(elements),
            Expression::Index(object, index) => self.visit_index(object, index),
            Expression::IndexSet(object, index, value) => self.visit_index_set(object, index, value),
            Expression::Object(fields) => self.visit_object(fields),
            Expression::Get(object, name) => self.visit_get(object, name),
            Expression::Set(object, name, value) => self.visit_set(object, name, value),
            Expression::This => self.visit_this(),
            Expression::Super(method) => self.visit_super(method),
            Expression::SuperCall(method, args) => self.visit_super_call(method, args),
            Expression::Function(params, body) => self.visit_function(params, body),
            Expression::Return(value) => self.visit_return(value),
        }
    }

    pub fn execute(&mut self, stmt: &Statment) -> Option<DataTypes> {
        if let Some(ret) = self.environment.borrow().return_val.clone() {
            return Some(ret);
        }
        match stmt {
            Statment::ExpressionStatement(exp) => Some(self.visit_expr(exp)),
            Statment::PrintStatement(exp) => {
                let val = self.visit_expr(exp);
                match &val {
                    DataTypes::Str(s) if s == "__tui_rendered__" => {}
                    DataTypes::Str(s) => println!("{s}"),
                    other => println!("{other}"),
                }
                None
            }
            Statment::LetStatement(name, exp) => {
                let val = self.visit_expr(exp);
                self.environment
                    .borrow_mut()
                    .define(name.text.clone(), val.clone());
                Some(val)
            }
            Statment::IfStatement(condition, then_branch, else_branch) => {
                let cond_val = self.visit_expr(condition);
                let is_truthy = match cond_val {
                    DataTypes::Bool(b) => b,
                    DataTypes::Number(n) => n != 0.0,
                    DataTypes::Str(s) => !s.is_empty(),
                    _ => false,
                };
                if is_truthy {
                    self.execute(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.execute(else_stmt)
                } else {
                    None
                }
            }
            Statment::WhileStatement(condition, body) => {
                loop {
                    let cond_val = self.visit_expr(condition);
                    let is_truthy = match cond_val {
                        DataTypes::Bool(b) => b,
                        DataTypes::Number(n) => n != 0.0,
                        DataTypes::Str(s) => !s.is_empty(),
                        _ => false,
                    };
                    if !is_truthy {
                        break;
                    }
                    if let Some(ret) = self.execute(body) {
                        if self.environment.borrow().return_val.is_some() {
                            return Some(ret);
                        }
                    }
                }
                None
            }
            Statment::ForStatement(init, condition, increment, body) => {
                if let Some(init_stmt) = init {
                    self.execute(init_stmt);
                }
                loop {
                    let should_continue = match condition {
                        Some(cond) => match self.visit_expr(cond) {
                            DataTypes::Bool(b) => b,
                            DataTypes::Number(n) => n != 0.0,
                            DataTypes::Str(s) => !s.is_empty(),
                            _ => false,
                        },
                        None => true,
                    };
                    if !should_continue {
                        break;
                    }
                    if let Some(ret) = self.execute(body) {
                        if self.environment.borrow().return_val.is_some() {
                            return Some(ret);
                        }
                    }
                    if let Some(inc_expr) = increment {
                        self.visit_expr(inc_expr);
                    }
                }
                None
            }
            Statment::BlockStatement(statements) => {
                let mut last_val = None;
                for stmt in statements {
                    if let Some(v) = self.execute(stmt) {
                        last_val = Some(v);
                    }
                    if self.environment.borrow().return_val.is_some() {
                        break;
                    }
                }
                last_val
            }
            Statment::ClassStatement(name_token, methods, superclass_token) => {
                let class_name = name_token.text.clone();
                let mut class_methods = std::collections::HashMap::new();
                let parent_env = self.environment.clone();
                for method_stmt in methods {
                    // Handle both direct Function and Assign-wrapped Function (from fn declaration)
                    let (method_name, params, body) =
                        if let Statment::ExpressionStatement(Expression::Function(params, body)) =
                            method_stmt
                        {
                            let method_name = params.first().cloned().unwrap_or_default();
                            (method_name, params.clone(), body.clone())
                        } else if let Statment::ExpressionStatement(Expression::Assign(
                            name,
                            expr,
                        )) = method_stmt
                        {
                            if let Expression::Function(params, body) = expr.as_ref() {
                                (name.clone(), params.clone(), body.clone())
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        };
                    let capture = Rc::new(RefCell::new(Environment::with_parent(
                        HashMap::new(),
                        parent_env.clone(),
                    )));
                    let lox_function = LoxFunction {
                        name: method_name.clone(),
                        params: params.clone(),
                        body: body.clone(),
                        closure: capture,
                        is_initializer: method_name == "init",
                    };
                    class_methods.insert(method_name, lox_function);
                }
                // Resolve superclass if present
                let superclass = if let Some(super_token) = superclass_token {
                    match self.environment.borrow().get(&super_token.text) {
                        Some(DataTypes::Class(c)) => Some(c.clone()),
                        _ => None,
                    }
                } else {
                    None
                };
                // Create LoxClass and store in environment
                let lox_class =
                    Rc::new(LoxClass::new(class_name.clone(), class_methods, superclass));
                self.environment
                    .borrow_mut()
                    .define(class_name, DataTypes::Class(lox_class));
                None
            }
            Statment::TestStatement(_, body) => self.execute(body),
        }
    }
}

fn bin_expr<D>(exp_a: DataTypes, exp_b: DataTypes, fun: impl FnOnce(D, D) -> D) -> DataTypes
where
    D: From<DataTypes> + Into<DataTypes>,
{
    // let arg_1 = D::from(exp_a);
    // let arg_2 = D::from(exp_b);
    fun(D::from(exp_a), D::from(exp_b)).into()
}

pub struct PrettyPrint;
impl ExpressionVisitor<String> for PrettyPrint {
    fn visit_binary(&mut self, left: &Expression, operator: &Token, right: &Expression) -> String {
        format!(
            "({} {} {})",
            operator.text,
            self.visit_expr(left.clone()),
            self.visit_expr(right.clone())
        )
    }

    fn visit_unary(&mut self, operator: &Token, right: &Expression) -> String {
        format!("({} {})", operator.text, self.visit_expr(right.clone()))
    }

    fn visit_literal(&mut self, token: &Token) -> String {
        token
            .literal
            .as_ref()
            .expect("INTERNAL ERROR: no literal is None")
            .print()
    }

    fn visit_grouping(&mut self, expression: &Expression) -> String {
        format!("(group {})", self.visit_expr(expression.clone()))
    }

    fn visit_assign(&mut self, name: &str, value: &Expression) -> String {
        format!("(assign {} {})", name, self.visit_expr(value.clone()))
    }

    fn visit_call(&mut self, name: &str, args: &[Expression]) -> String {
        let arg_strs: Vec<String> = args.iter().map(|a| self.visit_expr(a.clone())).collect();
        format!("(call {} {})", name, arg_strs.join(" "))
    }

    fn visit_array(&mut self, elements: &[Expression]) -> String {
        let elem_strs: Vec<String> = elements
            .iter()
            .map(|e| self.visit_expr(e.clone()))
            .collect();
        format!("[{}]", elem_strs.join(", "))
    }

    fn visit_index(&mut self, object: &Expression, index: &Expression) -> String {
        format!(
            "(index {} {})",
            self.visit_expr(object.clone()),
            self.visit_expr(index.clone())
        )
    }

    fn visit_index_set(
        &mut self,
        object: &Expression,
        index: &Expression,
        value: &Expression,
    ) -> String {
        format!(
            "(index-set {} {} {})",
            self.visit_expr(object.clone()),
            self.visit_expr(index.clone()),
            self.visit_expr(value.clone())
        )
    }

    fn visit_object(&mut self, fields: &[(String, Expression)]) -> String {
        let field_strs: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("{}: {}", k, self.visit_expr(v.clone())))
            .collect();
        format!("{{{}}}", field_strs.join(", "))
    }

    fn visit_get(&mut self, object: &Expression, name: &str) -> String {
        format!("(get {} {})", self.visit_expr(object.clone()), name)
    }

    fn visit_function(&mut self, params: &[String], _body: &Statment) -> String {
        format!("(fn {} ...)", params.join(", "))
    }

    fn visit_return(&mut self, value: &Option<Box<Expression>>) -> String {
        match value {
            Some(expr) => format!("(return {})", self.visit_expr(*expr.clone())),
            None => "(return)".to_string(),
        }
    }

    fn visit_set(&mut self, object: &Expression, name: &str, value: &Expression) -> String {
        format!(
            "(set {} {} {})",
            self.visit_expr(object.clone()),
            name,
            self.visit_expr(value.clone())
        )
    }

    fn visit_this(&mut self) -> String {
        "(this)".to_string()
    }

    fn visit_super(&mut self, method: &str) -> String {
        format!("(super.{})", method)
    }

    fn visit_super_call(&mut self, method: &str, args: &[Expression]) -> String {
        let arg_strs: Vec<String> = args.iter().map(|a| self.visit_expr(a.clone())).collect();
        format!("(super.{}.{})", method, arg_strs.join(" "))
    }
}

impl PrettyPrint {
    fn visit_expr(&mut self, e: Expression) -> String {
        match e {
            Expression::Binary(left, op, right) => self.visit_binary(&left, &op, &right),
            Expression::Unary(op, right) => self.visit_unary(&op, &right),
            Expression::Literal(token) => self.visit_literal(&token),
            Expression::Grouping(exp) => self.visit_grouping(&exp),
            Expression::Assign(name, value) => self.visit_assign(&name, &value),
            Expression::Call(name, args) => self.visit_call(&name, &args),
            Expression::Array(elements) => self.visit_array(&elements),
            Expression::Index(object, index) => self.visit_index(&object, &index),
            Expression::IndexSet(object, index, value) => {
                self.visit_index_set(&object, &index, &value)
            }
            Expression::Object(fields) => self.visit_object(&fields),
            Expression::Get(object, name) => self.visit_get(&object, &name),
            Expression::Set(object, name, value) => self.visit_set(&object, &name, &value),
            Expression::This => self.visit_this(),
            Expression::Super(method) => self.visit_super(&method),
            Expression::SuperCall(method, args) => self.visit_super_call(&method, &args),
            Expression::Function(params, body) => self.visit_function(&params, &body),
            Expression::Return(value) => self.visit_return(&value),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Statment {
    ExpressionStatement(Expression),
    PrintStatement(Expression),
    LetStatement(Token, Expression), // name, initializer
    IfStatement(Expression, Box<Statment>, Option<Box<Statment>>), // condition, then_branch, else_branch
    WhileStatement(Expression, Box<Statment>),                     // condition, body
    ForStatement(
        Option<Box<Statment>>,
        Option<Expression>,
        Option<Expression>,
        Box<Statment>,
    ), // init, condition, increment, body
    BlockStatement(Vec<Statment>), // For indented blocks with multiple statements
    ClassStatement(Token, Vec<Statment>, Option<Token>), // name, methods, superclass
    TestStatement(String, Box<Statment>), // test name, test body block
}

pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: tokens.into_iter().peekable(),
        }
    }
    pub fn parse(&mut self) -> Result<Vec<Statment>, Errors> {
        let mut statements: Vec<Statment> = Vec::new();
        // Skip leading newlines
        while self.check(TokenType::NEWLINE) {
            self.advance();
        }
        // Skip any leading DEDENT tokens at top level
        while self.check(TokenType::DEDENT) {
            self.advance();
        }
        while self
            .tokens
            .peek()
            .is_some_and(|t| t.token_type != TokenType::EOF)
        {
            // Skip any DEDENT or NEWLINE tokens between statements at top level
            while self.check(TokenType::DEDENT) || self.check(TokenType::NEWLINE) {
                self.advance();
            }
            if self.check(TokenType::EOF) {
                break;
            }
            statements.push(self.statement()?);
        }
        Ok(statements)
    }
    fn statement(&mut self) -> Result<Statment, Errors> {
        while self.check(TokenType::NEWLINE) {
            self.advance();
        }
        if self.match_token(TokenType::TEST) {
            let name_token = self.consume(TokenType::STRING, "Expect test description string")?;
            let test_name = match name_token.literal {
                Some(LiteralType::String(s)) => s,
                _ => name_token.text,
            };
            self.match_token(TokenType::COLON);
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            self.consume(
                TokenType::INDENT,
                "Expect indented block after test description",
            )?;
            let body = self.indented_block()?;
            return Ok(Statment::TestStatement(test_name, Box::new(body)));
        }

        if self.match_token(TokenType::PRINT) {
            let exp = self.expression()?;
            // Consume optional NEWLINE
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            return Ok(Statment::PrintStatement(exp));
        }
        if self.match_token(TokenType::LET) {
            let name = self.consume(TokenType::IDENTIFIER, "Expect variable name")?;
            let mut initializer = None;
            if self.match_token(TokenType::EQUAL) {
                initializer = Some(self.expression()?);
            }
            // Consume optional NEWLINE
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            return Ok(Statment::LetStatement(
                name,
                initializer.unwrap_or(Expression::Literal(Token::new(
                    TokenType::NIL,
                    "nil".to_string(),
                    None,
                    1,
                ))),
            ));
        }
        if self.match_token(TokenType::IF) {
            let condition = self.expression()?;
            self.match_token(TokenType::COLON);
            // Consume NEWLINE after condition
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            // Expect INDENT
            self.consume(TokenType::INDENT, "Expect indented block after 'if'")?;
            let then_branch = Box::new(self.indented_block()?);
            // indented_block() already consumes the DEDENT

            // Consume NEWLINE before else
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }

            let else_branch = if self.match_token(TokenType::ELSE) {
                self.match_token(TokenType::COLON);
                // Consume NEWLINE after else
                while self.check(TokenType::NEWLINE) {
                    self.advance();
                }
                // Expect INDENT
                self.consume(TokenType::INDENT, "Expect indented block after 'else'")?;
                let else_block = Box::new(self.indented_block()?);
                // indented_block() already consumes the DEDENT
                Some(else_block)
            } else {
                None
            };
            return Ok(Statment::IfStatement(condition, then_branch, else_branch));
        }
        if self.match_token(TokenType::WHILE) {
            let condition = self.expression()?;
            self.match_token(TokenType::COLON);
            // Consume NEWLINE after condition
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            // Expect INDENT
            self.consume(TokenType::INDENT, "Expect indented block after 'while'")?;
            let body = Box::new(self.indented_block()?);
            // indented_block() already consumes the DEDENT
            return Ok(Statment::WhileStatement(condition, body));
        }
        if self.match_token(TokenType::FOR) {
            // For loop: for init; condition; increment
            // Now using newlines instead of semicolons
            // Parse init (optional)
            let init = if self.check(TokenType::NEWLINE) || self.check(TokenType::INDENT) {
                // No init if newline or indent starts immediately
                None
            } else if self.match_token(TokenType::LET) {
                Some(Box::new(self.statement()?))
            } else {
                let exp = self.expression()?;
                Some(Box::new(Statment::ExpressionStatement(exp)))
            };

            // Consume NEWLINE after init
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }

            // Parse condition (optional)
            let condition = if self.check(TokenType::NEWLINE) || self.check(TokenType::INDENT) {
                None
            } else {
                Some(self.expression()?)
            };

            // Consume NEWLINE after condition
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }

            // Parse increment (optional)
            let increment = if self.check(TokenType::NEWLINE) || self.check(TokenType::INDENT) {
                None
            } else {
                Some(self.expression()?)
            };

            // Consume NEWLINE before body
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }

            // Expect INDENT
            self.consume(TokenType::INDENT, "Expect indented block after 'for'")?;
            let body = Box::new(self.indented_block()?);
            // indented_block() already consumes the DEDENT
            return Ok(Statment::ForStatement(init, condition, increment, body));
        }
        if self.match_token(TokenType::RETURN) {
            let value = if self.check(TokenType::NEWLINE)
                || self.check(TokenType::DEDENT)
                || self.is_at_end()
            {
                None
            } else {
                Some(Box::new(self.expression()?))
            };
            // Consume optional newline
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            return Ok(Statment::ExpressionStatement(Expression::Return(value)));
        }
        if self.match_token(TokenType::FN) {
            // Function declaration: fn name(params) { body } or fn name(params) => expr
            let name_token = self.consume(TokenType::IDENTIFIER, "Expect function name")?;
            let func_name = name_token.text.clone();
            let func_expr = self.function_literal(Some(name_token))?;
            // Store function in environment with name
            let assign_expr = Expression::Assign(func_name, Box::new(func_expr));
            // Consume optional newline
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            return Ok(Statment::ExpressionStatement(assign_expr));
        }
        if self.match_token(TokenType::CLASS) {
            let name_token = self.consume(TokenType::IDENTIFIER, "Expect class name")?;
            // Check for inheritance: class Derived < Base
            let superclass = if self.match_token(TokenType::LESS) {
                let superclass_name =
                    self.consume(TokenType::IDENTIFIER, "Expect superclass name")?;
                Some(superclass_name)
            } else {
                None
            };
            self.match_token(TokenType::COLON);
            // Consume optional NEWLINE
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            // Expect INDENT
            self.consume(TokenType::INDENT, "Expect indented block after class")?;
            let methods_stmt = self.indented_block()?;
            // indented_block() already consumes the DEDENT
            // Extract statements from BlockStatement
            let methods = match methods_stmt {
                Statment::BlockStatement(stmts) => stmts,
                _ => vec![],
            };
            return Ok(Statment::ClassStatement(name_token, methods, superclass));
        }
        let exp = self.expression()?;
        while self.check(TokenType::NEWLINE) {
            self.advance();
        }
        Ok(Statment::ExpressionStatement(exp))
    }
    fn indented_block(&mut self) -> Result<Statment, Errors> {
        let mut statements = Vec::new();
        // Skip leading newlines
        while self.check(TokenType::NEWLINE) {
            self.advance();
        }
        while !self.check(TokenType::DEDENT) && !self.is_at_end() {
            statements.push(self.statement()?);
        }
        // Consume the single DEDENT for this block
        if self.check(TokenType::DEDENT) {
            self.advance();
        }
        Ok(Statment::BlockStatement(statements))
    }

    fn check(&mut self, token_type: TokenType) -> bool {
        self.tokens
            .peek()
            .is_some_and(|t| t.token_type == token_type)
    }

    fn is_at_end(&mut self) -> bool {
        self.tokens
            .peek()
            .is_some_and(|t| t.token_type == TokenType::EOF)
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<Token, Errors> {
        if self.check(token_type) {
            return Ok(self.tokens.next().unwrap());
        }
        if let Some(tok) = self.tokens.peek() {
            let found = if tok.token_type == TokenType::NEWLINE {
                "newline".to_string()
            } else if tok.token_type == TokenType::EOF {
                "end of file".to_string()
            } else if tok.token_type == TokenType::INDENT {
                "indent".to_string()
            } else if tok.token_type == TokenType::DEDENT {
                "dedent".to_string()
            } else {
                tok.text.clone()
            };
            return Err(Errors::SyntaxError {
                line: tok.span.start_line.max(tok.line),
                col: tok.span.start_col.max(1),
                found,
                message: message.to_string(),
                help: None,
            });
        }
        Err(Errors::SyntaxError {
            line: 1,
            col: 1,
            found: "end of file".to_string(),
            message: message.to_string(),
            help: None,
        })
    }

    fn expression(&mut self) -> Result<Expression, Errors> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expression, Errors> {
        let exp = self.logic_or()?;

        if let Some(eq_tok) = self.tokens.next_if(|t| t.token_type == TokenType::EQUAL) {
            let value = self.assignment()?;
            match exp {
                Expression::Literal(token) if token.token_type == TokenType::IDENTIFIER => {
                    return Ok(Expression::Assign(token.text, Box::new(value)));
                }
                Expression::Get(object, name) => {
                    return Ok(Expression::Set(object, name, Box::new(value)));
                }
                Expression::Index(object, index) => {
                    return Ok(Expression::IndexSet(object, index, Box::new(value)));
                }
                _ => {
                    return Err(Errors::SyntaxError {
                        line: eq_tok.span.start_line.max(eq_tok.line),
                        col: eq_tok.span.start_col.max(1),
                        found: "=".to_string(),
                        message: "Invalid assignment target".to_string(),
                        help: Some("Only variables, properties, and indexed elements can be assigned.".to_string()),
                    });
                }
            }
        }
        Ok(exp)
    }

    fn logic_or(&mut self) -> Result<Expression, Errors> {
        let mut exp = self.logic_and()?;
        while let Some(token) = self
            .tokens
            .next_if(|token| token.token_type == TokenType::OR)
        {
            let right = self.logic_and()?;
            exp = Expression::Binary(Box::new(exp), token, Box::new(right));
        }
        Ok(exp)
    }

    fn logic_and(&mut self) -> Result<Expression, Errors> {
        let mut exp = self.equality()?;
        while let Some(token) = self
            .tokens
            .next_if(|token| token.token_type == TokenType::AND)
        {
            let right = self.equality()?;
            exp = Expression::Binary(Box::new(exp), token, Box::new(right));
        }
        Ok(exp)
    }

    fn equality(&mut self) -> Result<Expression, Errors> {
        let mut exp: Expression = self.comp()?;

        while let Some(token) = self.tokens.next_if(|token| {
            [TokenType::BangEqual, TokenType::EqualEqual].contains(&token.token_type)
        }) {
            let expr_right = self.comp()?;
            exp = Expression::Binary(Box::new(exp), token, Box::new(expr_right))
        }
        Ok(exp)
    }

    fn comp(&mut self) -> Result<Expression, Errors> {
        let mut exp: Expression = self.term()?;

        while let Some(token) = self.tokens.next_if(|token| {
            [
                TokenType::GreaterEqual,
                TokenType::GREATER,
                TokenType::LESS,
                TokenType::LessEqual,
            ]
            .contains(&token.token_type)
        }) {
            let expr_right = self.term()?;
            exp = Expression::Binary(Box::new(exp), token, Box::new(expr_right))
        }
        Ok(exp)
    }

    fn term(&mut self) -> Result<Expression, Errors> {
        let mut exp: Expression = self.factor()?;

        while let Some(token) = self
            .tokens
            .next_if(|token| [TokenType::MINUS, TokenType::PLUS].contains(&token.token_type))
        {
            let expr_right = self.factor()?;
            exp = Expression::Binary(Box::new(exp), token, Box::new(expr_right))
        }
        Ok(exp)
    }

    fn factor(&mut self) -> Result<Expression, Errors> {
        let mut exp: Expression = self.unary()?;

        while let Some(token) = self
            .tokens
            .next_if(|token| [TokenType::SLASH, TokenType::STAR].contains(&token.token_type))
        {
            let expr_right = self.unary()?;
            exp = Expression::Binary(Box::new(exp), token, Box::new(expr_right))
        }
        Ok(exp)
    }

    fn unary(&mut self) -> Result<Expression, Errors> {
        if let Some(operator) = self
            .tokens
            .next_if(|token| [TokenType::BANG, TokenType::MINUS].contains(&token.token_type))
        {
            let exp_right = self.unary()?;
            return Ok(Expression::Unary(operator, Box::new(exp_right)));
        }

        self.primary()
    }

    fn primary(&mut self) -> Result<Expression, Errors> {
        if let Some(token) = self.tokens.next() {
            match token.token_type {
                TokenType::TRUE | TokenType::FALSE | TokenType::NIL => {
                    return Ok(Expression::Literal(token));
                }

                TokenType::NUMBER | TokenType::STRING => return Ok(Expression::Literal(token)),

                TokenType::LeftBracket => {
                    return self.array_literal();
                }

                TokenType::LeftBrace => {
                    return self.object_literal();
                }

                TokenType::FN => {
                    return self.function_literal(None);
                }

                TokenType::THIS => {
                    return Ok(Expression::This);
                }

                TokenType::SUPER => {
                    self.consume(TokenType::DOT, "Expect '.' after 'super'")?;
                    let method =
                        self.consume(TokenType::IDENTIFIER, "Expect superclass method name")?;
                    // Check if this is a method call: super.method()
                    if self.check(TokenType::LeftPara) {
                        self.consume(TokenType::LeftPara, "Expect '(' after method name")?;
                        let mut args = Vec::new();
                        if !self.check(TokenType::RightPara) {
                            loop {
                                args.push(self.expression()?);
                                if !self.match_token(TokenType::COMMA) {
                                    break;
                                }
                            }
                        }
                        self.consume(TokenType::RightPara, "Expect ')' after arguments")?;
                        return Ok(Expression::SuperCall(method.text, args));
                    }
                    return Ok(Expression::Super(method.text));
                }
                TokenType::IDENTIFIER => {
                    // Check if this is a function call (identifier followed by '(')
                    let name = token.text.clone();
                    let mut expr = Expression::Literal(token);
                    if self.check(TokenType::LeftPara) {
                        self.consume(TokenType::LeftPara, "Expect '(' after function name")?;
                        let mut args = Vec::new();
                        if !self.check(TokenType::RightPara) {
                            loop {
                                args.push(self.expression()?);
                                if !self.match_token(TokenType::COMMA) {
                                    break;
                                }
                            }
                        }
                        self.consume(TokenType::RightPara, "Expect ')' after arguments")?;
                        expr = Expression::Call(name, args);
                    }
                    // Handle index, get, and method call expressions
                    loop {
                        if self.match_token(TokenType::LeftBracket) {
                            let index = self.expression()?;
                            self.consume(TokenType::RightBracket, "Expect ']' after index")?;
                            expr = Expression::Index(Box::new(expr), Box::new(index));
                        } else if self.match_token(TokenType::DOT) {
                            let name_token = self
                                .consume(TokenType::IDENTIFIER, "Expect property name after '.'")?;
                            // Check if this is a method call
                            if self.check(TokenType::LeftPara) {
                                self.consume(TokenType::LeftPara, "Expect '(' after method name")?;
                                let mut args = Vec::new();
                                if !self.check(TokenType::RightPara) {
                                    loop {
                                        args.push(self.expression()?);
                                        if !self.match_token(TokenType::COMMA) {
                                            break;
                                        }
                                    }
                                }
                                self.consume(TokenType::RightPara, "Expect ')' after arguments")?;
                                // Prepend the object as first argument for method call
                                args.insert(0, expr);
                                expr = Expression::Call(name_token.text, args);
                            } else {
                                // Just a property access
                                expr = Expression::Get(Box::new(expr), name_token.text);
                            }
                        } else {
                            break;
                        }
                    }
                    return Ok(expr);
                }
                TokenType::LeftPara => {
                    let exp = self.expression()?;
                    if let Some(_token) = self
                        .tokens
                        .next_if(|t| t.token_type == TokenType::RightPara)
                    {
                        return Ok(Expression::Grouping(Box::new(exp)));
                    } else {
                        return Err(Errors::UnterminatedGroup);
                    }
                }
                other => {
                    let found = if other == TokenType::NEWLINE {
                        "newline".to_string()
                    } else if other == TokenType::EOF {
                        "end of file".to_string()
                    } else if other == TokenType::INDENT {
                        "indent".to_string()
                    } else if other == TokenType::DEDENT {
                        "dedent".to_string()
                    } else {
                        token.text.clone()
                    };
                    let help = if other == TokenType::COLON {
                        Some("Pseudo uses indentation without ':' for blocks (though optional ':' on block headers is allowed).".to_string())
                    } else {
                        Some("Expected an expression (identifier, literal, '(', '[', or '{').".to_string())
                    };
                    return Err(Errors::SyntaxError {
                        line: token.span.start_line.max(token.line),
                        col: token.span.start_col.max(1),
                        found,
                        message: "Unexpected token in expression".to_string(),
                        help,
                    });
                }
            }
        }
        Err(Errors::SyntaxError {
            line: 1,
            col: 1,
            found: "end of file".to_string(),
            message: "Unexpected end of input while parsing expression".to_string(),
            help: None,
        })
    }

    fn array_literal(&mut self) -> Result<Expression, Errors> {
        let mut elements = Vec::new();
        while self.check(TokenType::NEWLINE)
            || self.check(TokenType::INDENT)
            || self.check(TokenType::DEDENT)
        {
            self.advance();
        }
        if !self.check(TokenType::RightBracket) {
            loop {
                while self.check(TokenType::NEWLINE)
                    || self.check(TokenType::INDENT)
                    || self.check(TokenType::DEDENT)
                {
                    self.advance();
                }
                elements.push(self.expression()?);
                while self.check(TokenType::NEWLINE)
                    || self.check(TokenType::INDENT)
                    || self.check(TokenType::DEDENT)
                {
                    self.advance();
                }
                if !self.match_token(TokenType::COMMA) {
                    break;
                }
            }
        }
        while self.check(TokenType::NEWLINE)
            || self.check(TokenType::INDENT)
            || self.check(TokenType::DEDENT)
        {
            self.advance();
        }
        self.consume(TokenType::RightBracket, "Expect ']' after array elements")?;
        Ok(Expression::Array(elements))
    }

    fn object_literal(&mut self) -> Result<Expression, Errors> {
        let mut fields = Vec::new();
        while self.check(TokenType::NEWLINE)
            || self.check(TokenType::INDENT)
            || self.check(TokenType::DEDENT)
        {
            self.advance();
        }
        if !self.check(TokenType::RightBrace) {
            loop {
                while self.check(TokenType::NEWLINE)
                    || self.check(TokenType::INDENT)
                    || self.check(TokenType::DEDENT)
                {
                    self.advance();
                }
                let key_token = self.consume(TokenType::IDENTIFIER, "Expect property name")?;
                self.consume(TokenType::COLON, "Expect ':' after property name")?;
                while self.check(TokenType::NEWLINE)
                    || self.check(TokenType::INDENT)
                    || self.check(TokenType::DEDENT)
                {
                    self.advance();
                }
                let value = self.expression()?;
                fields.push((key_token.text, value));
                while self.check(TokenType::NEWLINE)
                    || self.check(TokenType::INDENT)
                    || self.check(TokenType::DEDENT)
                {
                    self.advance();
                }
                if !self.match_token(TokenType::COMMA) {
                    break;
                }
            }
        }
        while self.check(TokenType::NEWLINE)
            || self.check(TokenType::INDENT)
            || self.check(TokenType::DEDENT)
        {
            self.advance();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after object fields")?;
        Ok(Expression::Object(fields))
    }

    fn function_literal(&mut self, name_token: Option<Token>) -> Result<Expression, Errors> {
        let _name = name_token.map(|t| t.text);
        self.consume(TokenType::LeftPara, "Expect '(' after 'fn'")?;
        let mut params = Vec::new();
        if !self.check(TokenType::RightPara) {
            loop {
                let param = self.consume(TokenType::IDENTIFIER, "Expect parameter name")?;
                params.push(param.text);
                if !self.match_token(TokenType::COMMA) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightPara, "Expect ')' after parameters")?;

        // Check for expression body (=>) or block body
        if self.match_token(TokenType::ARROW) {
            // Consume optional NEWLINE before checking for INDENT
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            // Check if next token is INDENT (block body) or expression
            if self.check(TokenType::INDENT) {
                // Block body after =>
                self.consume(TokenType::INDENT, "Expect indented block after function")?;
                let body = self.indented_block()?;
                // indented_block() already consumes the DEDENT
                Ok(Expression::Function(params, Box::new(body)))
            } else {
                // Expression body
                let expr = self.expression()?;
                let body = Statment::ExpressionStatement(expr);
                Ok(Expression::Function(params, Box::new(body)))
            }
        } else {
            // Block body - expect INDENT (allow optional ':')
            self.match_token(TokenType::COLON);
            // Consume optional NEWLINE
            while self.check(TokenType::NEWLINE) {
                self.advance();
            }
            self.consume(TokenType::INDENT, "Expect indented block after function")?;
            let body = self.indented_block()?;
            // indented_block() already consumes the DEDENT
            Ok(Expression::Function(params, Box::new(body)))
        }
    }

    fn match_token(&mut self, token_type: TokenType) -> bool {
        self.tokens
            .next_if(|t| t.token_type == token_type)
            .is_some()
    }

    fn advance(&mut self) -> Option<Token> {
        self.tokens.next()
    }
}
fn json_to_datatypes(val: &serde_json::Value) -> DataTypes {
    match val {
        serde_json::Value::Null => DataTypes::Number(0.0),
        serde_json::Value::Bool(b) => DataTypes::Bool(*b),
        serde_json::Value::Number(n) => DataTypes::Number(n.as_f64().unwrap_or(0.0) as f32),
        serde_json::Value::String(s) => DataTypes::Str(s.clone()),
        serde_json::Value::Array(arr) => {
            DataTypes::Array(arr.iter().map(json_to_datatypes).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_datatypes(v));
            }
            DataTypes::Object(map)
        }
    }
}
//     self.tokens
//         .peek()
//         .is_some_and(|src_token| src_token.token_type == token_type)
// }

// struct Binary {
//     left: Expression,
//     operator: BinaryOperator,
//     right: Expression,
// }
// enum BinaryOperator {}
// struct Uniary {}
// struct Grouping {}
// struct Literal {}
