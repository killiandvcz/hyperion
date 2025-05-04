//! Parser for HyperionQL
//!
//! This module provides functionality to parse query strings into AST.

use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};

use crate::errors::{Result, StoreError};
use crate::path::Path;
use crate::value::Value;
use crate::ql::ast::{Query, Operation, Expression};
use std::str::FromStr;

#[derive(Parser)]
#[grammar = "ql/grammar.pest"]
pub struct HyperionQLParser;

pub fn parse_query(input: &str) -> Result<Query> {
    // Parse with pest
    let pairs = HyperionQLParser::parse(Rule::main, input)
        .map_err(|e| StoreError::InvalidOperation(format!("Parse error: {}", e)))?;
    
    // Convert to AST
    parse_query_ast(pairs)
}

fn parse_query_ast(pairs: Pairs<Rule>) -> Result<Query> {
    // Trouver la paire 'query'
    let query_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::InvalidOperation("Empty query".to_string()))?;
    
    let mut operations = Vec::new();
    let mut return_expr = None;
    
    // Itérer sur les parties de la requête
    for pair in query_pair.into_inner() {
        match pair.as_rule() {
            Rule::operation => {
                let operation = parse_operation(pair)?;
                operations.push(operation);
            },
            Rule::return_stmt => {
                let expr_pair = pair.into_inner().next().unwrap();
                let expr = parse_expression(expr_pair)?;
                return_expr = Some(expr);
            },
            _ => {}
        }
    }
    
    // Vérifier qu'il y a une expression de retour
    let return_expr = return_expr.ok_or_else(|| 
        StoreError::InvalidOperation("Missing return statement".to_string()))?;
    
    Ok(Query {
        operations,
        return_expr,
    })
}

fn parse_operation(pair: Pair<Rule>) -> Result<Operation> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::assignment => {
            let mut inner_pairs = inner.into_inner();
            let path_pair = inner_pairs.next().unwrap();
            let expr_pair = inner_pairs.next().unwrap();
            
            let path = parse_path(path_pair)?;
            let expr = parse_expression(expr_pair)?;
            
            Ok(Operation::Assignment {
                path,
                expression: expr,
            })
        },
        Rule::delete_op => {
            let path_pair = inner.into_inner().next().unwrap();
            let path = parse_path(path_pair)?;
            
            Ok(Operation::Delete { path })
        },
        _ => Err(StoreError::InvalidOperation(
            format!("Unexpected operation type: {:?}", inner.as_rule())
        )),
    }
}

fn parse_expression(pair: Pair<Rule>) -> Result<Expression> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::literal => parse_literal(inner),
        Rule::path => {
            let path = parse_path(inner)?;
            Ok(Expression::Path(path))
        },
        // Supprimer ou modifier cette partie pour ne plus traiter entity_expr
        // Rule::entity_expr => { ... }
        Rule::function_call => {
            // Si on veut conserver la compatibilité avec entity() pour l'instant,
            // on pourrait ajouter une vérification spéciale ici
            let mut inner_pairs = inner.into_inner();
            let name_pair = inner_pairs.next().unwrap();
            let name = name_pair.as_str().to_string();
            
            let mut arguments = Vec::new();
            for arg_pair in inner_pairs {
                let arg = parse_expression(arg_pair)?;
                arguments.push(arg);
            }
            
            // Pour rétrocompatibilité temporaire
            if name == "entity" && arguments.len() == 1 {
                if let Expression::Path(path) = &arguments[0] {
                    return Ok(Expression::Path(path.clone()));
                }
            }
            
            Ok(Expression::FunctionCall { name, arguments })
        },
        _ => Err(StoreError::InvalidOperation(
            format!("Unexpected expression type: {:?}", inner.as_rule())
        )),
    }
}

fn parse_literal(pair: Pair<Rule>) -> Result<Expression> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::string => {
            // Extraire la valeur de la chaîne sans les guillemets
            let s = inner.as_str();
            let s = &s[1..s.len()-1]; // Enlever les guillemets
            Ok(Expression::Literal(Value::String(s.to_string())))
        },
        Rule::number => {
            let n = inner.as_str().parse::<f64>()
                .map_err(|_| StoreError::InvalidOperation(
                    format!("Invalid number: {}", inner.as_str())
                ))?;
                
            // Déterminer si c'est un entier ou un flottant
            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                Ok(Expression::Literal(Value::Integer(n as i64)))
            } else {
                Ok(Expression::Literal(Value::Float(n)))
            }
        },
        Rule::boolean => {
            let b = inner.as_str() == "true";
            Ok(Expression::Literal(Value::Boolean(b)))
        },
        Rule::null => {
            Ok(Expression::Literal(Value::Null))
        },
        _ => Err(StoreError::InvalidOperation(
            format!("Unexpected literal type: {:?}", inner.as_rule())
        )),
    }
}

fn parse_path(pair: Pair<Rule>) -> Result<Path> {
    let path_str = pair.as_str();
    Path::from_str(path_str).map_err(|e| StoreError::InvalidOperation(format!("Path error: {}", e)))
}