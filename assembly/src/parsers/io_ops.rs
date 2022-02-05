use super::{
    parse_element_param, parse_int_param, push_value, validate_op_len, AssemblyError, Operation,
    Token,
};

// PUSHING VALUES ONTO THE STACK (PUSH)
// ================================================================================================

/// Pushes constant, environment, or non-deterministic (advice) inputs onto the stack as
/// specified by the operation variant and its parameter(s).
///
/// *CONSTANTS: `push.a`*
/// Pushes the immediate value `a` onto the stack.
///
/// *ENVIRONMENT: `push.env.{var}`*
/// Pushes the value of the specified environment variable onto the top of the stack. Currently, the
/// only environment input is `sdepth`.
///
/// *NON-DETERMINISTIC (ADVICE): `push.adv.n`*
/// Removes the next `n` values from the advice tape and pushes them onto the stack. The number of
/// items that can be read from the advice tape is limited to 16.
///
/// # Errors
///
/// Returns an `AssemblyError` if the op param is invalid, malformed, or doesn't match an expected
/// push instruction
pub fn parse_push(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    if op.num_parts() < 2 {
        return Err(AssemblyError::invalid_op(op));
    }
    if op.parts()[0] != "push" {
        return Err(AssemblyError::unexpected_token(
            op,
            "push.{adv.n|env.var|a}",
        ));
    }

    match op.parts()[1] {
        "adv" => parse_push_adv(span_ops, op),
        "env" => parse_push_env(span_ops, op),
        _ => parse_push_constant(span_ops, op),
    }
}

/// Pushes a word (4 elements) onto the stack from an absolute location in random access memory or
/// from local procedure memory as specified by the operation variant and its parameter.
///
/// *RANDOM ACCESS MEMORY: `pushw.mem`, `pushw.mem.a`*
/// Reads a word (4 elements) from memory and pushes it onto the stack by appending `LOADW` and
/// required stack manipulations to the span block. If no memory address is specified, it is assumed
/// to be on top of the stack. Otherwise, the provided address will be pushed so it is on top of the
/// stack when `LOADW` is executed. The memory address will be removed from the stack by `LOADW`.
///
/// *LOCAL PROCEDURE VARIABLES: `pushw.local.i`*
/// Reads a word (4 elements) from local memory at index `i` and pushes it onto the stack.
///
/// # Errors
///
/// Returns an `AssemblyError` if the op param is invalid, malformed, or doesn't match an expected
/// `pushw` instruction.
pub fn parse_pushw(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // validate op
    validate_op_len(op, 2, 0, 1)?;
    if op.parts()[0] != "pushw" {
        return Err(AssemblyError::unexpected_token(
            op,
            "pushw.{mem|mem.a|local.i}",
        ));
    }

    match op.parts()[1] {
        "mem" => parse_mem_read(span_ops, op),
        "local" => parse_local_read(span_ops, op),
        _ => Err(AssemblyError::invalid_op(op)),
    }
}

// REMOVING VALUES FROM THE STACK (POP)
// ================================================================================================

/// Pops a word (4 elements) from the stack and store it at an absolute memory location or in local
/// procedure memory as specified by the operation variant and its parameter.
///
/// *RANDOM ACCESS MEMORY: `popw.mem`, `popw.mem.a`*
/// Pops the top 4 elements off the stack and stores them at an absolute address in memory by
/// appending `STOREW` and required stack manipulations to the span block. If no memory address is
/// provided as a parameter, the address is assumed to be on top of the stack. Otherwise, the
/// provided address will be pushed so it is on top of the stack when `STOREW` is executed. The
/// memory address will be removed from the stack by `STOREW`.
///
/// *LOCAL PROCEDURE VARIABLES: `popw.local.i`*
/// Pops the top 4 elements off the stack and stores them in local memory at index `i`.
///
/// # Errors
///
/// Returns an `AssemblyError` if the op param is invalid, malformed, or doesn't match an expected
/// `popw` instruction.
pub fn parse_popw(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // validate op
    validate_op_len(op, 2, 0, 1)?;
    if op.parts()[0] != "popw" {
        return Err(AssemblyError::unexpected_token(
            op,
            "popw.{mem|mem.a|local.i}",
        ));
    }

    match op.parts()[1] {
        "mem" => parse_mem_write(span_ops, op),
        "local" => parse_local_write(span_ops, op),
        _ => Err(AssemblyError::invalid_op(op)),
    }
}

// OVERWRITING VALUES ON THE STACK (LOAD)
// ================================================================================================

/// Overwrites the top 4 elements of the stack with a word (4 elements) loaded from either the
/// advice tape, an absolute location in random access memory, or procedure locals as specified by
/// the operation variant and its parameter.
///
/// *NON-DETERMINISTIC (ADVICE): `loadw.adv`*
/// Removes the next word (4 elements) from the advice tape and overwrites the top 4 elements of the
/// stack with it. Fails if the advice tape has fewer than 4 elements.
///
/// *RANDOM ACCESS MEMORY: `loadw.mem`, `loadw.mem.a`*
/// Reads a word (4 elements) from memory and overwrites the top 4 elements of the stack with it by
/// appending `LOADW` and required stack manipulations to the span block. If no memory address is
/// specified, the address is assumed to be on top of the stack. Otherwise, the provided address
/// will be pushed so it is on top of the stack when `LOADW` is executed. The memory address will be
/// removed from the stack by `LOADW`.
///
/// *LOCAL PROCEDURE VARIABLES: `loadw.local.i`*
/// Reads a word (4 elements) from local memory at index `i` and overwrites the top 4 elements of
/// the stack with it.
///
/// # Errors
///
/// Returns an `AssemblyError` if the op param is invalid, malformed, or doesn't match an expected
/// `loadw` instruction.
pub fn parse_loadw(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // validate op
    validate_op_len(op, 2, 0, 1)?;
    if op.parts()[0] != "loadw" {
        return Err(AssemblyError::unexpected_token(
            op,
            "loadw.{adv|mem|mem.a|local.i}",
        ));
    }

    match op.parts()[1] {
        "adv" => {
            // ensure that no parameter exists
            if op.num_parts() > 2 {
                return Err(AssemblyError::extra_param(op));
            }

            // load a word from the advice tape
            span_ops.push(Operation::ReadW);
            Ok(())
        }
        "mem" => parse_mem_read(span_ops, op),
        "local" => parse_local_read(span_ops, op),
        _ => Err(AssemblyError::invalid_op(op)),
    }
}

// SAVING STACK VALUES WITHOUT REMOVING THEM (STORE)
// ================================================================================================

/// Stores the top 4 elements of the stack at an absolute memory location or in local procedure
/// memory, as specified by the operation variant and its parameter. If a memory address is provided
/// via the stack, it will be removed first. At the end of the operation, all elements will remain
/// on the stack.
///
/// *RANDOM ACCESS MEMORY: `storew.mem`, `storew.mem.a`*
/// Stores the top 4 elements of the stack at an absolute address in memory by appending `STOREW`
/// and required stack manipulations to the span block. If no memory address is provided as a
/// parameter, the address is assumed to be on top of the stack. Otherwise, the provided address
/// will be pushed so it is on top of the stack when `STOREW` is executed.  The memory address will
/// be removed from the stack by `STOREW`.
///
/// *LOCAL PROCEDURE VARIABLES: `storew.local.i`*
/// Stores the top 4 elements of the stack in local memory at index `i`.
///
/// # Errors
///
/// Returns an `AssemblyError` if the op param is invalid, malformed, or doesn't match an expected
/// `storew` instruction.
pub fn parse_storew(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // validate op
    validate_op_len(op, 2, 0, 1)?;
    if op.parts()[0] != "storew" {
        return Err(AssemblyError::unexpected_token(
            op,
            "storew.{mem|mem.a|local.i}",
        ));
    }

    match op.parts()[1] {
        "mem" => parse_mem_write(span_ops, op),
        "local" => parse_local_write(span_ops, op),
        _ => Err(AssemblyError::invalid_op(op)),
    }
}

// HELPERS - CONSTANT INPUTS
// ================================================================================================

/// Appends a `PUSH` operation to the span block.
///
/// In cases when the immediate value is 0, `PUSH` operation is replaced with `PAD`. Also, in cases
/// when immediate value is 1, `PUSH` operation is replaced with `PAD INCR` because in most cases
/// this will be more efficient than doing a `PUSH`.
///
/// # Errors
///
/// This function expects an assembly op with exactly one immediate value that is a valid field
/// element in decimal or hexadecimal representation. It will return an error if the immediate
/// value is invalid or missing. It will also return an error if the op token is malformed or
/// doesn't match the expected instruction.
fn parse_push_constant(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // validate op
    validate_op_len(op, 1, 1, 1)?;
    if op.parts()[0] != "push" {
        return Err(AssemblyError::unexpected_token(op, "push.{param}"));
    }

    // update the span block
    let value = parse_element_param(op, 1)?;
    push_value(span_ops, value);

    Ok(())
}

// HELPERS - ENVIRONMENT INPUTS
// ================================================================================================

/// Appends machine operations to the current span block according to the requested environment
/// assembly instruction.
///
/// `push.env.sdepth` pushes the current depth of the stack onto the top of the stack, which is
/// handled directly by the `SDEPTH` operation.
///
/// # Errors
///
/// This function expects a valid assembly environment op that specifies the environment input to
/// be handled. It will return an error if the assembly instruction is malformed or the environment
/// input is unrecognized.
fn parse_push_env(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // validate the operation
    validate_op_len(op, 3, 0, 0)?;
    if op.parts()[1] != "env" {
        return Err(AssemblyError::unexpected_token(op, "push.env.{var}"));
    }

    // update the span block
    match op.parts()[2] {
        "sdepth" => {
            span_ops.push(Operation::SDepth);
        }
        _ => return Err(AssemblyError::invalid_op(op)),
    }

    Ok(())
}

// HELPERS - NON-DETERMINISTIC INPUTS
// ================================================================================================
const ADVICE_READ_LIMIT: u32 = 16;

/// Appends the number of `READ` operations specified by the operation's immediate value
/// to the span block. This pushes the specified number of items from the advice tape onto the
/// stack. It limits the number of items that can be read from the advice tape at a time to 16.
///
/// # Errors
///
/// Returns an `AssemblyError` if the instruction is invalid, malformed, missing a required
/// parameter, or does not match the expected operation. Returns an `invalid_param` `AssemblyError`
/// if the parameter for `push.adv` is not a decimal value.
fn parse_push_adv(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    // do basic validation common to all advice operations
    validate_op_len(op, 2, 1, 1)?;
    if op.parts()[1] != "adv" {
        return Err(AssemblyError::unexpected_token(op, "push.adv.n"));
    }

    // parse and validate the parameter as the number of items to read from the advice tape
    // it must be between 1 and ADVICE_READ_LIMIT, inclusive, since adv.push.0 is a no-op
    let n = parse_int_param(op, 2, 1, ADVICE_READ_LIMIT)?;

    // read n items from the advice tape and push then onto the stack
    for _ in 0..n {
        span_ops.push(Operation::Read);
    }

    Ok(())
}

// HELPERS - RANDOM ACCESS MEMORY
// ================================================================================================

/// Translates the `pushw.mem` and `loadw.mem` assembly ops to the system's `LOADW` memory read
/// operation.
///
/// If the op provides an address (e.g. `pushw.mem.a`), it must be pushed to the stack directly
/// before the `LOADW` operation. For `loadw.mem`, `LOADW` can be used directly. For `pushw.mem`,
/// space for 4 new elements on the stack must be made first, using `PAD`. Then, if the memory
/// address was provided via the stack (not as part of the memory op) it must be moved to the top.
///
/// # Errors
///
/// This function expects a memory read assembly operation that has already been validated. If
/// called without validation, it could yield incorrect results or return an `AssemblyError`.
fn parse_mem_read(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    if op.parts()[0] == "pushw" {
        // make space for the new elements
        for _ in 0..4 {
            span_ops.push(Operation::Pad);
        }

        // put the memory address on top of the stack
        if op.num_parts() == 2 {
            // move the memory address to the top of the stack
            span_ops.push(Operation::MovUp4);
        } else {
            // parse the provided memory address and push it onto the stack
            let address = parse_element_param(op, 2)?;
            span_ops.push(Operation::Push(address));
        }
    } else if op.num_parts() == 3 {
        push_mem_addr(span_ops, op)?;
    }

    // load from the memory address on top of the stack
    span_ops.push(Operation::LoadW);

    Ok(())
}

/// Translates the `popw.mem` and `storew.mem` assembly ops to the system's `STOREW` memory write
/// operation.
///
/// If the op provides an address (e.g. `popw.mem.a`), it must be pushed to the stack directly
/// before the `STOREW` operation. For `storew.mem`, `STOREW` can be used directly. For `popw.mem`,
/// the stack must `DROP` the top 4 elements after they are written to memory.
///
/// # Errors
///
/// This function expects a memory write assembly operation that has already been validated. If
/// called without validation, it could yield incorrect results or return an `AssemblyError`.
fn parse_mem_write(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    if op.num_parts() == 3 {
        push_mem_addr(span_ops, op)?;
    }

    span_ops.push(Operation::StoreW);

    if op.parts()[0] == "popw" {
        for _ in 0..4 {
            span_ops.push(Operation::Drop);
        }
    }

    Ok(())
}

/// Parses a provided memory address and pushes it onto the stack.
///
/// # Errors
///
/// This function will return an `AssemblyError` if the address parameter does not exist.
fn push_mem_addr(span_ops: &mut Vec<Operation>, op: &Token) -> Result<(), AssemblyError> {
    let address = parse_element_param(op, 2)?;
    span_ops.push(Operation::Push(address));

    Ok(())
}

// HELPERS - LOCAL MEMORY FOR PROCEDURE VARIABLES
// ================================================================================================
fn parse_local_read(_span_ops: &mut Vec<Operation>, _op: &Token) -> Result<(), AssemblyError> {
    unimplemented!();
}

fn parse_local_write(_span_ops: &mut Vec<Operation>, _op: &Token) -> Result<(), AssemblyError> {
    unimplemented!();
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::{BaseElement, FieldElement};

    #[test]
    fn push() {
        let mut span_ops: Vec<Operation> = Vec::new();
        let op_0 = Token::new("push.0", 0);
        let op_1 = Token::new("push.1", 0);
        let op_dec = Token::new("push.135", 0);
        let op_hex = Token::new("push.0x7b", 0);
        let expected = vec![
            Operation::Pad,
            Operation::Pad,
            Operation::Incr,
            Operation::Push(BaseElement::new(135)),
            Operation::Push(BaseElement::new(123)),
        ];

        parse_push(&mut span_ops, &op_0).expect("Failed to parse push.0");
        parse_push(&mut span_ops, &op_1).expect("Failed to parse push.1");
        parse_push(&mut span_ops, &op_dec).expect("Failed to parse push of decimal element 123");
        parse_push(&mut span_ops, &op_hex).expect("Failed to parse push of hex element 0x7b");

        assert_eq!(span_ops, expected);
    }

    #[test]
    fn push_invalid() {
        // fails when immediate value is invalid or missing
        let mut span_ops: Vec<Operation> = Vec::new();
        let param_idx = 0;

        // value missing
        let op_no_val = Token::new("push", param_idx);
        let expected = AssemblyError::missing_param(&op_no_val);
        assert_eq!(parse_push(&mut span_ops, &op_no_val).unwrap_err(), expected);

        // invalid value
        let op_val_invalid = Token::new("push.abc", param_idx);
        let expected = AssemblyError::invalid_param(&op_val_invalid, 1);
        assert_eq!(
            parse_push(&mut span_ops, &op_val_invalid).unwrap_err(),
            expected
        );

        // extra value
        let op_extra_val = Token::new("push.0.1", param_idx);
        let expected = AssemblyError::extra_param(&op_extra_val);
        assert_eq!(
            parse_push(&mut span_ops, &op_extra_val).unwrap_err(),
            expected
        );

        // wrong operation passed to parsing function
        let op_mismatch = Token::new("pushw.0", param_idx);
        let expected = AssemblyError::unexpected_token(&op_mismatch, "push.{param}");
        assert_eq!(
            parse_push(&mut span_ops, &op_mismatch).unwrap_err(),
            expected
        )
    }

    #[test]
    fn pushw() {
        // pushes a word of 4 immediate values in decimal or hexadecimal onto the stack
        let mut span_ops: Vec<Operation> = Vec::new();
        let op = Token::new("pushw.1.23.0x1C8.0", 0);
        let expected = vec![
            Operation::Pad,
            Operation::Incr,
            Operation::Push(BaseElement::new(23)),
            Operation::Push(BaseElement::new(456)),
            Operation::Pad,
        ];
        parse_pushw(&mut span_ops, &op).expect("Failed to parse pushw");

        assert_eq!(span_ops, expected);
    }

    #[test]
    fn pushw_invalid() {
        // fails when immediate values are invalid or missing
        let mut span_ops: Vec<Operation> = Vec::new();
        let param_idx = 0;

        // no values
        let op_no_vals = Token::new("pushw", param_idx);
        let expected = AssemblyError::missing_param(&op_no_vals);
        assert_eq!(
            parse_pushw(&mut span_ops, &op_no_vals).unwrap_err(),
            expected
        );

        // insufficient values provided
        let op_val_missing = Token::new("pushw.0.1.2", param_idx);
        let expected = AssemblyError::missing_param(&op_val_missing);
        assert_eq!(
            parse_pushw(&mut span_ops, &op_val_missing).unwrap_err(),
            expected
        );

        // invalid value
        let op_val_invalid = Token::new("pushw.0.1.2.abc", param_idx);
        let expected = AssemblyError::invalid_param(&op_val_invalid, 4);
        assert_eq!(
            parse_pushw(&mut span_ops, &op_val_invalid).unwrap_err(),
            expected
        );

        // extra value
        let op_extra_val = Token::new("pushw.0.1.2.3.4", param_idx);
        let expected = AssemblyError::extra_param(&op_extra_val);
        assert_eq!(
            parse_pushw(&mut span_ops, &op_extra_val).unwrap_err(),
            expected
        );

        // wrong operation passed to parsing function
        let op_mismatch = Token::new("push.0.1.2.3", param_idx);
        let expected =
            AssemblyError::unexpected_token(&op_mismatch, "pushw.{param}.{param}.{param}.{param}");
        assert_eq!(
            parse_pushw(&mut span_ops, &op_mismatch).unwrap_err(),
            expected
        )
    }

    #[test]
    fn env_sdepth() {
        // pushes the current depth of the stack onto the top of the stack
        let mut span_ops = vec![Operation::Push(BaseElement::ONE); 8];
        let op = Token::new("env.sdepth", 0);
        let mut expected = span_ops.clone();
        expected.push(Operation::SDepth);

        parse_env(&mut span_ops, &op).expect("Failed to parse env.sdepth with empty stack");
        assert_eq!(span_ops, expected);
    }

    #[test]
    fn env_invalid() {
        // fails when env op variant is invalid or missing or has too many immediate values
        let mut span_ops: Vec<Operation> = Vec::new();
        let param_idx = 0;

        // missing env var
        let op_no_val = Token::new("env", param_idx);
        let expected = AssemblyError::invalid_op(&op_no_val);
        assert_eq!(parse_env(&mut span_ops, &op_no_val).unwrap_err(), expected);

        // invalid env var
        let op_val_invalid = Token::new("env.invalid", param_idx);
        let expected = AssemblyError::invalid_op(&op_val_invalid);
        assert_eq!(
            parse_env(&mut span_ops, &op_val_invalid).unwrap_err(),
            expected
        );

        // extra value
        let op_extra_val = Token::new("env.sdepth.0", param_idx);
        let expected = AssemblyError::extra_param(&op_extra_val);
        assert_eq!(
            parse_env(&mut span_ops, &op_extra_val).unwrap_err(),
            expected
        );

        // wrong operation passed to parsing function
        let op_mismatch = Token::new("push.sdepth", param_idx);
        let expected = AssemblyError::unexpected_token(&op_mismatch, "env.{param}");
        assert_eq!(
            parse_env(&mut span_ops, &op_mismatch).unwrap_err(),
            expected
        )
    }

    #[test]
    fn adv_push() {
        // remove n items from the advice tape and push them onto the stack
        let mut span_ops: Vec<Operation> = Vec::new();
        let op = Token::new("adv.push.4", 0);
        let expected = vec![Operation::Read; 4];

        parse_adv(&mut span_ops, &op).expect("Failed to parse adv.push.4");
        assert_eq!(span_ops, expected);
    }

    #[test]
    fn adv_loadw() {
        // replace the top 4 elements of the stack with 4 elements from the advice tape
        let mut span_ops: Vec<Operation> = Vec::new();
        let op = Token::new("adv.loadw", 0);
        let expected = vec![Operation::ReadW];

        parse_adv(&mut span_ops, &op).expect("Failed to parse adv.loadw");
        assert_eq!(span_ops, expected);
    }

    #[test]
    fn adv_invalid_instruction() {
        // fails when the instruction is malformed or unrecognized
        let mut span_ops: Vec<Operation> = Vec::new();
        let param_idx = 0;

        // missing variant
        let op_no_variant = Token::new("adv", param_idx);
        let expected = AssemblyError::invalid_op(&op_no_variant);
        assert_eq!(
            parse_adv(&mut span_ops, &op_no_variant).unwrap_err(),
            expected
        );

        // missing value
        let op_no_val = Token::new("adv.push", param_idx);
        let expected = AssemblyError::missing_param(&op_no_val);
        assert_eq!(parse_adv(&mut span_ops, &op_no_val).unwrap_err(), expected);

        // extra value to push
        let op_extra_val = Token::new("adv.push.2.2", param_idx);
        let expected = AssemblyError::extra_param(&op_extra_val);
        assert_eq!(
            parse_adv(&mut span_ops, &op_extra_val).unwrap_err(),
            expected
        );

        // extra value to loadw
        let op_extra_val = Token::new("adv.loadw.0", param_idx);
        let expected = AssemblyError::extra_param(&op_extra_val);
        assert_eq!(
            parse_adv(&mut span_ops, &op_extra_val).unwrap_err(),
            expected
        );

        // bad adv op variant passed to parsing function
        let op_wrong_variant = Token::new("adv.read", param_idx);
        let expected = AssemblyError::invalid_op(&op_wrong_variant);
        assert_eq!(
            parse_adv(&mut span_ops, &op_wrong_variant).unwrap_err(),
            expected
        );

        // wrong prefix in op passed to pasing function
        let op_wrong_prefix = Token::new("mem.push.2", param_idx);
        let expected = AssemblyError::unexpected_token(&op_wrong_prefix, "adv.{push.n|loadw}");
        assert_eq!(
            parse_adv(&mut span_ops, &op_wrong_prefix).unwrap_err(),
            expected
        );
    }

    #[test]
    fn adv_invalid_param() {
        // fails when immediate value to advice push operation is invalid
        let mut span_ops: Vec<Operation> = Vec::new();
        let param_idx = 0;

        // invalid value - char
        let op_invalid_char = Token::new("adv.push.a", param_idx);
        let expected = AssemblyError::invalid_param(&op_invalid_char, 2);
        assert_eq!(
            parse_adv(&mut span_ops, &op_invalid_char).unwrap_err(),
            expected
        );

        // invalid value - hexadecimal
        let op_invalid_hex = Token::new("adv.push.0x10", param_idx);
        let expected = AssemblyError::invalid_param(&op_invalid_hex, 2);
        assert_eq!(
            parse_adv(&mut span_ops, &op_invalid_hex).unwrap_err(),
            expected
        );

        // parameter out of bounds
        let reason = format!(
            "parameter value must be greater than or equal to {} and less than or equal to {}",
            1, ADVICE_READ_LIMIT
        );
        // less than lower bound
        let op_lower_bound = Token::new("adv.push.0", param_idx);
        let expected = AssemblyError::invalid_param_with_reason(&op_lower_bound, 2, &reason);
        assert_eq!(
            parse_adv(&mut span_ops, &op_lower_bound).unwrap_err(),
            expected
        );

        // greater than upper bound
        let inst_str = format!("adv.push.{}", ADVICE_READ_LIMIT + 1);
        let op_upper_bound = Token::new(&inst_str, param_idx);
        let expected = AssemblyError::invalid_param_with_reason(&op_upper_bound, 2, &reason);
        assert_eq!(
            parse_adv(&mut span_ops, &op_upper_bound).unwrap_err(),
            expected
        );
    }

    #[test]
    fn mem_pop() {
        // stores the top 4 elements of the stack in memory
        // then removes those 4 elements from the top of the stack

        // test pop with memory address on top of the stack
        let mut span_ops: Vec<Operation> = Vec::new();
        let op_mem_pop = Token::new("mem.pop", 0);
        let expected = vec![
            Operation::StoreW,
            Operation::Drop,
            Operation::Drop,
            Operation::Drop,
            Operation::Drop,
        ];
        parse_mem(&mut span_ops, &op_mem_pop).expect("Failed to parse mem.pop");
        assert_eq!(&span_ops, &expected);

        // test pop with memory address provided directly (address 0)
        let mut span_ops_addr: Vec<Operation> = Vec::new();
        let op_pop_addr = Token::new("mem.pop.0", 0);
        let expected_addr = vec![
            Operation::Push(BaseElement::ZERO),
            Operation::StoreW,
            Operation::Drop,
            Operation::Drop,
            Operation::Drop,
            Operation::Drop,
        ];

        parse_mem(&mut span_ops_addr, &op_pop_addr).expect("Failed to parse mem.pop.0");

        assert_eq!(&span_ops_addr, &expected_addr);
    }

    #[test]
    fn mem_store() {
        // stores the top 4 elements of the stack in memory

        // test store with memory address on top of the stack
        let mut span_ops: Vec<Operation> = Vec::new();
        let op_store = Token::new("mem.store", 0);
        let expected = vec![Operation::StoreW];

        parse_mem(&mut span_ops, &op_store).expect("Failed to parse mem.store");

        assert_eq!(&span_ops, &expected);

        // test store with memory address provided directly (address 0)
        let mut span_ops_addr: Vec<Operation> = Vec::new();
        let op_store_addr = Token::new("mem.store.0", 0);
        let expected_addr = vec![Operation::Push(BaseElement::ZERO), Operation::StoreW];

        parse_mem(&mut span_ops_addr, &op_store_addr)
            .expect("Failed to parse mem.store.0 with adddress (address provided by op)");

        assert_eq!(&span_ops_addr, &expected_addr);
    }

    #[test]
    fn mem_push() {
        // reads a word from memory and pushes it onto the stack

        // test push with memory address on top of stack
        let mut span_ops: Vec<Operation> = Vec::new();
        let op_push = Token::new("mem.push", 0);
        let expected = vec![
            Operation::Pad,
            Operation::Pad,
            Operation::Pad,
            Operation::Pad,
            Operation::MovUp4,
            Operation::LoadW,
        ];

        parse_mem(&mut span_ops, &op_push).expect("Failed to parse mem.push");

        assert_eq!(&span_ops, &expected);

        // test push with memory address provided directly (address 0)
        let mut span_ops_addr: Vec<Operation> = Vec::new();
        let op_push_addr = Token::new("mem.push.0", 0);
        let expected_addr = vec![
            Operation::Pad,
            Operation::Pad,
            Operation::Pad,
            Operation::Pad,
            Operation::Push(BaseElement::ZERO),
            Operation::LoadW,
        ];

        parse_mem(&mut span_ops_addr, &op_push_addr)
            .expect("Failed to parse mem.push.0 (address provided by op)");

        assert_eq!(&span_ops_addr, &expected_addr);
    }

    #[test]
    fn mem_load() {
        // reads a word from memory and overwrites the top 4 stack elements

        // test load with memory address on top of stack
        let mut span_ops: Vec<Operation> = Vec::new();
        let op_push = Token::new("mem.load", 0);
        let expected = vec![Operation::LoadW];

        parse_mem(&mut span_ops, &op_push).expect("Failed to parse mem.load");

        assert_eq!(&span_ops, &expected);

        // test load with memory address provided directly (address 0)
        let mut span_ops_addr: Vec<Operation> = Vec::new();
        let op_load_addr = Token::new("mem.load.0", 0);
        let expected_addr = vec![Operation::Push(BaseElement::ZERO), Operation::LoadW];

        parse_mem(&mut span_ops_addr, &op_load_addr)
            .expect("Failed to parse mem.load.0 (address provided by op)");

        assert_eq!(&span_ops_addr, &expected_addr);
    }

    #[test]
    fn mem_invalid() {
        // fails when mem op variant is invalid or missing or has too many immediate values
        let mut span_ops: Vec<Operation> = Vec::new();
        let param_idx = 0;

        // missing variant
        let op_missing = Token::new("mem", param_idx);
        let expected = AssemblyError::invalid_op(&op_missing);
        assert_eq!(parse_mem(&mut span_ops, &op_missing).unwrap_err(), expected);

        // invalid variant
        let op_invalid = Token::new("mem.abc", param_idx);
        let expected = AssemblyError::invalid_op(&op_invalid);
        assert_eq!(parse_mem(&mut span_ops, &op_invalid).unwrap_err(), expected);

        // invalid param
        let op_val_invalid = Token::new("mem.push.a", param_idx);
        let expected = AssemblyError::invalid_param(&op_val_invalid, 2);
        assert_eq!(
            parse_mem(&mut span_ops, &op_val_invalid).unwrap_err(),
            expected
        );

        // extra value
        let op_extra_val = Token::new("mem.push.0.1", param_idx);
        let expected = AssemblyError::extra_param(&op_extra_val);
        assert_eq!(
            parse_mem(&mut span_ops, &op_extra_val).unwrap_err(),
            expected
        );

        // wrong operation passed to parsing function
        let op_mismatch = Token::new("adv.push.0", param_idx);
        let expected = AssemblyError::unexpected_token(&op_mismatch, "mem.{push|load|pop|store}");
        assert_eq!(
            parse_mem(&mut span_ops, &op_mismatch).unwrap_err(),
            expected
        );
    }
}
