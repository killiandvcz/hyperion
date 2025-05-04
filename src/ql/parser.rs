//! Parser for HyperionQL
//!
//! This module provides functionality to parse query strings into AST.

use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};

use crate::errors::{Result, StoreError};
use crate::path::Path;
use crate::value::Value;
use crate::ql::ast::{Query, Operation, Expression, ComparisonOperator, LogicalOperator, Condition, WhereClause};
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
    
    // Vérifier qu'il y a au moins des opérations si pas de return
    if operations.is_empty() && return_expr.is_none() {
        return Err(StoreError::InvalidOperation("Query must contain at least one operation or a return statement".to_string()));
    }
    
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

// Modifier la fonction parse_expression pour gérer les nouveaux types d'expressions
fn parse_expression(pair: Pair<Rule>) -> Result<Expression> {
    let mut inner_pairs = pair.into_inner();
    
    // Première partie: l'expression primaire
    let primary_expr_pair = inner_pairs.next()
        .ok_or_else(|| StoreError::InvalidOperation("Missing primary expression".to_string()))?;
    
    let primary_expr = parse_primary_expression(primary_expr_pair)?;
    
    // Vérifier s'il y a une clause where
    if let Some(where_clause_pair) = inner_pairs.next() {
        if where_clause_pair.as_rule() == Rule::where_clause {
            let where_clause = parse_where_clause(where_clause_pair)?;
            
            return Ok(Expression::Filtered {
                base: Box::new(primary_expr),
                where_clause,
            });
        }
    }
    
    // Pas de clause where, retourner l'expression primaire
    Ok(primary_expr)
}

// Nouvelle fonction pour parser une expression primaire
fn parse_primary_expression(pair: Pair<Rule>) -> Result<Expression> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::literal => parse_literal(inner),
        Rule::path => {
            let path = parse_path(inner)?;
            Ok(Expression::Path(path))
        },
        Rule::their_path => {
            let mut segments = Vec::new();
            
            // On itère sur toutes les paires internes (les segments de chemin)
            for segment_pair in inner.into_inner() {
                if segment_pair.as_rule() == Rule::path_segment {
                    segments.push(segment_pair.as_str().to_string());
                }
            }

            println!("Parsed their_path segments: {:?}", segments);
            
            Ok(Expression::TheirPath(segments))
        },
        Rule::function_call => {
            let mut inner_pairs = inner.into_inner();
            let name_pair = inner_pairs.next().unwrap();
            let name = name_pair.as_str().to_string();
            
            let mut arguments = Vec::new();
            for arg_pair in inner_pairs {
                let arg = parse_primary_expression(arg_pair)?;
                arguments.push(arg);
            }
            
            Ok(Expression::FunctionCall { name, arguments })
        },
        _ => Err(StoreError::InvalidOperation(
            format!("Unexpected primary expression type: {:?}", inner.as_rule())
        )),
    }
}

fn parse_where_clause(pair: Pair<Rule>) -> Result<WhereClause> {
    let where_expr_pair = pair.into_inner().next().unwrap();
    
    let mut conditions_pairs = where_expr_pair.into_inner();
    
    // La première condition est obligatoire
    let first_condition_pair = conditions_pairs.next()
        .ok_or_else(|| StoreError::InvalidOperation("Missing condition in where clause".to_string()))?;
    
    let first_condition = parse_condition(first_condition_pair)?;
    
    // Collecter les conditions supplémentaires avec leurs opérateurs logiques
    let mut additional_conditions = Vec::new();
    
    while let Some(op_pair) = conditions_pairs.next() {
        if op_pair.as_rule() != Rule::logical_op {
            return Err(StoreError::InvalidOperation(
                format!("Expected logical operator, found {:?}", op_pair.as_rule())
            ));
        }
        
        let operator = parse_logical_operator(op_pair)?;
        
        let cond_pair = conditions_pairs.next()
            .ok_or_else(|| StoreError::InvalidOperation("Missing condition after logical operator".to_string()))?;
        
        let condition = parse_condition(cond_pair)?;
        
        additional_conditions.push((operator, condition));
    }
    
    Ok(WhereClause {
        first_condition,
        additional_conditions,
    })
}


fn parse_condition(pair: Pair<Rule>) -> Result<Condition> {
    let mut inner_pairs = pair.into_inner();
    
    let left_pair = inner_pairs.next().unwrap();
    let left = Box::new(parse_primary_expression(left_pair)?);
    
    let op_pair = inner_pairs.next().unwrap();
    let operator = parse_comparison_operator(op_pair)?;
    
    let right_pair = inner_pairs.next().unwrap();
    let right = Box::new(parse_primary_expression(right_pair)?);
    
    Ok(Condition {
        left,
        operator,
        right,
    })
}

// Nouvelle fonction pour parser un opérateur de comparaison
fn parse_comparison_operator(pair: Pair<Rule>) -> Result<ComparisonOperator> {
    match pair.as_str() {
        "==" => Ok(ComparisonOperator::Equal),
        "!=" => Ok(ComparisonOperator::NotEqual),
        "<" => Ok(ComparisonOperator::LessThan),
        "<=" => Ok(ComparisonOperator::LessThanOrEqual),
        ">" => Ok(ComparisonOperator::GreaterThan),
        ">=" => Ok(ComparisonOperator::GreaterThanOrEqual),
        _ => Err(StoreError::InvalidOperation(
            format!("Unknown comparison operator: {}", pair.as_str())
        )),
    }
}

// Nouvelle fonction pour parser un opérateur logique
fn parse_logical_operator(pair: Pair<Rule>) -> Result<LogicalOperator> {
    match pair.as_str() {
        "&&" => Ok(LogicalOperator::And),
        "||" => Ok(LogicalOperator::Or),
        _ => Err(StoreError::InvalidOperation(
            format!("Unknown logical operator: {}", pair.as_str())
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
    let path_str = pair.as_str().trim();
    Path::from_str(path_str).map_err(|e| StoreError::InvalidOperation(format!("Path error: {}", e)))
}