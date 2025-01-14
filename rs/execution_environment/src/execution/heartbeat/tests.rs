use crate::execution::heartbeat::CanisterHeartbeatError;
use crate::execution::test_utilities::{wat_compilation_cost, ExecutionTestBuilder};
use assert_matches::assert_matches;
use ic_ic00_types::CanisterStatusType;
use ic_interfaces::execution_environment::{HypervisorError, TrapCode};
use ic_replicated_state::{page_map::PAGE_SIZE, CanisterStatus};
use ic_types::methods::SystemMethod;
use ic_types::NumBytes;

#[test]
fn heartbeat_is_executed() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_heartbeat") unreachable)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    let err = test
        .heartbeat_or_timer(canister_id, SystemMethod::CanisterHeartbeat)
        .unwrap_err();
    assert_eq!(
        err,
        CanisterHeartbeatError::CanisterExecutionFailed(HypervisorError::Trapped(
            TrapCode::Unreachable
        ))
    );
}

#[test]
fn global_timer_is_executed() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_global_timer") unreachable)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    let err = test
        .heartbeat_or_timer(canister_id, SystemMethod::CanisterGlobalTimer)
        .unwrap_err();
    assert_eq!(
        err,
        CanisterHeartbeatError::CanisterExecutionFailed(HypervisorError::Trapped(
            TrapCode::Unreachable
        ))
    );
}

#[test]
fn heartbeat_produces_heap_delta() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_heartbeat")
                (i32.store (i32.const 10) (i32.const 10))
            )
            (memory (export "memory") 1)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    assert_eq!(NumBytes::from(0), test.state().metadata.heap_delta_estimate);
    test.heartbeat_or_timer(canister_id, SystemMethod::CanisterHeartbeat)
        .unwrap();
    assert_eq!(
        NumBytes::from((PAGE_SIZE) as u64),
        test.state().metadata.heap_delta_estimate
    );
}

#[test]
fn global_timer_produces_heap_delta() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_global_timer")
                (i32.store (i32.const 10) (i32.const 10))
            )
            (memory (export "memory") 1)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    assert_eq!(NumBytes::from(0), test.state().metadata.heap_delta_estimate);
    test.heartbeat_or_timer(canister_id, SystemMethod::CanisterGlobalTimer)
        .unwrap();
    assert_eq!(
        NumBytes::from((PAGE_SIZE) as u64),
        test.state().metadata.heap_delta_estimate
    );
}

#[test]
fn heartbeat_fails_gracefully_if_not_exported() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = "(module)";
    let canister_id = test.canister_from_wat(wat).unwrap();
    test.heartbeat_or_timer(canister_id, SystemMethod::CanisterHeartbeat)
        .unwrap();
    assert_eq!(NumBytes::from(0), test.state().metadata.heap_delta_estimate);
    assert_eq!(wat_compilation_cost(wat), test.executed_instructions());
}

#[test]
fn global_timer_fails_gracefully_if_not_exported() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = "(module)";
    let canister_id = test.canister_from_wat(wat).unwrap();
    test.heartbeat_or_timer(canister_id, SystemMethod::CanisterGlobalTimer)
        .unwrap();
    assert_eq!(NumBytes::from(0), test.state().metadata.heap_delta_estimate);
    assert_eq!(wat_compilation_cost(wat), test.executed_instructions());
}

#[test]
fn heartbeat_doesnt_run_if_canister_is_stopped() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_heartbeat") unreachable)
            (memory (export "memory") 1)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    test.stop_canister(canister_id);
    test.process_stopping_canisters();
    assert_eq!(
        CanisterStatus::Stopped,
        test.canister_state(canister_id).system_state.status
    );
    let err = test
        .heartbeat_or_timer(canister_id, SystemMethod::CanisterHeartbeat)
        .unwrap_err();
    assert_eq!(
        err,
        CanisterHeartbeatError::CanisterNotRunning {
            status: CanisterStatusType::Stopped,
        }
    );
}

#[test]
fn global_timer_doesnt_run_if_canister_is_stopped() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_global_timer") unreachable)
            (memory (export "memory") 1)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    test.stop_canister(canister_id);
    test.process_stopping_canisters();
    assert_eq!(
        CanisterStatus::Stopped,
        test.canister_state(canister_id).system_state.status
    );
    let err = test
        .heartbeat_or_timer(canister_id, SystemMethod::CanisterGlobalTimer)
        .unwrap_err();
    assert_eq!(
        err,
        CanisterHeartbeatError::CanisterNotRunning {
            status: CanisterStatusType::Stopped,
        }
    );
}

#[test]
fn heartbeat_doesnt_run_if_canister_is_stopping() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_heartbeat") unreachable)
            (memory (export "memory") 1)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    test.stop_canister(canister_id);
    assert_matches!(
        test.canister_state(canister_id).system_state.status,
        CanisterStatus::Stopping {
            call_context_manager: _,
            stop_contexts: _
        }
    );
    let err = test
        .heartbeat_or_timer(canister_id, SystemMethod::CanisterHeartbeat)
        .unwrap_err();
    assert_eq!(
        err,
        CanisterHeartbeatError::CanisterNotRunning {
            status: CanisterStatusType::Stopping,
        }
    );
}

#[test]
fn global_timer_doesnt_run_if_canister_is_stopping() {
    let mut test = ExecutionTestBuilder::new().build();
    let wat = r#"
        (module
            (func (export "canister_global_timer") unreachable)
            (memory (export "memory") 1)
        )"#;
    let canister_id = test.canister_from_wat(wat).unwrap();
    test.stop_canister(canister_id);
    assert_matches!(
        test.canister_state(canister_id).system_state.status,
        CanisterStatus::Stopping {
            call_context_manager: _,
            stop_contexts: _
        }
    );
    let err = test
        .heartbeat_or_timer(canister_id, SystemMethod::CanisterGlobalTimer)
        .unwrap_err();
    assert_eq!(
        err,
        CanisterHeartbeatError::CanisterNotRunning {
            status: CanisterStatusType::Stopping,
        }
    );
}
