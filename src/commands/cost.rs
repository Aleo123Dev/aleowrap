use std::{collections::HashMap, ops::Add, str::FromStr};

use anyhow::{Context, Result};
use rand::{rngs::StdRng, SeedableRng};
use serde_json;
use snarkvm::{
    prelude::{
        deployment_cost as vm_deployment_cost, execution_cost as vm_execution_cost,
        query::Query,
        store::{helpers::memory::{ConsensusMemory, BlockMemory}, ConsensusStore},
        Identifier, PrivateKey, ProgramID, VM,
    },
    synthesizer::{Process, Program},
};

use super::{deploy::resolve_imports, CurrentAleo, CurrentNetwork};

pub fn deployment_cost(program: &str, imports: Option<HashMap<String, String>>) -> Result<String> {
    let program = Program::from_str(program)?;
    let mut process = Process::<CurrentNetwork>::load().context("Error process load")?;
    let _ = resolve_imports(&mut process, &program, imports);
    let rng = &mut StdRng::from_entropy();

    println!("Creating deployment");
    // Generate the deployment
    let deployment = process
        .deploy::<CurrentAleo, _>(&program, rng)
        .context("Error process deploy")?;

    let (minimum_deployment_cost, (storage_cost, finalize_cost)) =
        vm_deployment_cost(&deployment).context("Error deployment_cost")?;

    let json_object = serde_json::json!({
        "minimum_deployment_cost":minimum_deployment_cost,
        "storage_cost":storage_cost,
        "finalize_cost":finalize_cost,
    });

    Ok(json_object.to_string())
}

pub fn execution_cost(
    private_key: &str,
    program_id: &str,
    function: &str,
    inputs: Vec<String>,
    query: Option<&str>,
) -> Result<String> {
    // // Initialize an RNG.
    let rng = &mut rand::thread_rng();

    // Initialize the VM.
    let store = ConsensusStore::<CurrentNetwork, ConsensusMemory<CurrentNetwork>>::open(None)?;
    let vm = VM::from(store)?;

    let private_key = PrivateKey::from_str(private_key)?;
    let program_id = ProgramID::from_str(program_id)?;
    let function_name = Identifier::from_str(function)?;
    let query = match query {
        Some(query) => query,
        None => "https://vm.aleo.org/api",
    };

    // Compute the authorization.
    let authorization = vm
        .authorize(&private_key, program_id, function_name, inputs, rng)
        .context("Error execution_cost vm authorize")?;

    let process = Process::<CurrentNetwork>::load().context("Error process load")?;

    let (_, mut trace) = process
        .execute::<CurrentAleo>(authorization)
        .context("Error process execute")?;

    let query = Query::<CurrentNetwork, BlockMemory<_>>::from(query);

    trace.prepare(query)?;

    let locator = program_id.to_string().add("/").add(function);
    let execution = trace
        .prove_execution::<CurrentAleo, _>(&locator, &mut StdRng::from_entropy())
        .context("execution_cost prove_execution load")?;

    let (minimum_deployment_cost, (storage_cost, finalize_cost)) =
        vm_execution_cost(&vm, &execution)?;

    let json_object = serde_json::json!({
        "minimum_deployment_cost":minimum_deployment_cost,
        "storage_cost":storage_cost,
        "finalize_cost":finalize_cost,
    });
    
    Ok(json_object.to_string())
}
