use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
    program_pack::{IsInitialized, Pack},
};
use solana_program::program::{invoke, invoke_signed};
use solana_program::system_instruction;
use spl_token::state::{Mint, Account};

// Define a struct to represent the token
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TokenAccount {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub amount: u64,
}

// Entry point of the program
entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;
    let rent = &Rent::from_account_info(next_account_info(accounts_iter)?)?;

    let (instruction, rest) = instruction_data.split_at(1);
    match instruction[0] {
        0 => create_token(program_id, accounts_iter, rest, rent, account),
        1 => create_token_account(program_id, accounts_iter, rest, rent, account),
        2 => mint_tokens(program_id, accounts_iter, rest, account),
        3 => transfer_tokens(program_id, accounts_iter, rest, account),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn create_token(
    program_id: &Pubkey,
    accounts: &mut std::slice::Iter<AccountInfo>,
    rest: &[u8],
    rent: &Rent,
    account: &AccountInfo,
) -> ProgramResult {
    let mint_account = next_account_info(accounts)?;

    let mint_data_len = Mint::LEN;
    if account.data_len() < mint_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let rent_exempt_balance = rent.minimum_balance(mint_data_len);
    if account.lamports() < rent_exempt_balance {
        return Err(ProgramError::InsufficientFunds);
    }

    let mint_authority = next_account_info(accounts)?;
    let freeze_authority = next_account_info(accounts)?;

    let mut mint_info = Mint::unpack_unchecked(&mint_account.data.borrow())?;
    mint_info.mint_authority = COption::Some(*mint_authority.key);
    mint_info.freeze_authority = COption::Some(*freeze_authority.key);
    mint_info.is_initialized = true;

    Mint::pack(mint_info, &mut mint_account.data.borrow_mut())?;
    Ok(())
}

fn create_token_account(
    program_id: &Pubkey,
    accounts: &mut std::slice::Iter<AccountInfo>,
    rest: &[u8],
    rent: &Rent,
    account: &AccountInfo,
) -> ProgramResult {
    let token_account = next_account_info(accounts)?;
    let owner_account = next_account_info(accounts)?;

    let account_data_len = Account::LEN;
    if account.data_len() < account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let rent_exempt_balance = rent.minimum_balance(account_data_len);
    if account.lamports() < rent_exempt_balance {
        return Err(ProgramError::InsufficientFunds);
    }

    let mut token_info = Account::unpack_unchecked(&token_account.data.borrow())?;
    token_info.owner = *owner_account.key;
    token_info.is_initialized = true;

    Account::pack(token_info, &mut token_account.data.borrow_mut())?;
    Ok(())
}

fn mint_tokens(
    program_id: &Pubkey,
    accounts: &mut std::slice::Iter<AccountInfo>,
    rest: &[u8],
    account: &AccountInfo,
) -> ProgramResult {
    let mint_account = next_account_info(accounts)?;
    let token_account = next_account_info(accounts)?;
    let mint_authority = next_account_info(accounts)?;

    let amount = u64::from_le_bytes(rest.try_into().unwrap());

    let mut mint_info = Mint::unpack(&mint_account.data.borrow())?;
    let mut token_info = Account::unpack(&token_account.data.borrow())?;

    if mint_info.mint_authority != COption::Some(*mint_authority.key) {
        return Err(ProgramError::IncorrectAuthority);
    }

    token_info.amount += amount;
    Account::pack(token_info, &mut token_account.data.borrow_mut())?;
    Ok(())
}

fn transfer_tokens(
    program_id: &Pubkey,
    accounts: &mut std::slice::Iter<AccountInfo>,
    rest: &[u8],
    account: &AccountInfo,
) -> ProgramResult {
    let source_account = next_account_info(accounts)?;
    let destination_account = next_account_info(accounts)?;
    let owner_account = next_account_info(accounts)?;

    let amount = u64::from_le_bytes(rest.try_into().unwrap());

    let mut source_info = Account::unpack(&source_account.data.borrow())?;
    let mut destination_info = Account::unpack(&destination_account.data.borrow())?;

    if source_info.owner != *owner_account.key {
        return Err(ProgramError::IncorrectAuthority);
    }

    if source_info.amount < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    source_info.amount -= amount;
    destination_info.amount += amount;

    Account::pack(source_info, &mut source_account.data.borrow_mut())?;
    Account::pack(destination_info, &mut destination_account.data.borrow_mut())?;
    Ok(())
}
