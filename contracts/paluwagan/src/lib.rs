#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Env, Address, Vec,
    token, Symbol,
};

#[contract]
pub struct PaluwaganNiJuan;

// ==================== STORAGE KEYS ====================
const GROUPS_KEY: Symbol = symbol_short!("groups");
const MEMBERS_KEY: Symbol = symbol_short!("members");
const CONTRIBUTIONS_KEY: Symbol = symbol_short!("contribs");
const NEXT_GROUP_ID_KEY: Symbol = symbol_short!("nextid");

// ==================== DATA STRUCTURES ====================

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupStatus {
    Active = 0,
    Completed = 1,
    Paused = 2,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PaluwagGroup {
    pub group_id: u64,
    pub organizer: Address,
    pub token_contract: Address,
    pub member_count: u32,
    pub contribution_amount: i128,
    pub frequency_days: u32,
    pub status: GroupStatus,
    pub created_at: u64,
    pub current_cycle: u32,
    pub current_recipient_index: u32,
    pub contributions_this_cycle: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Member {
    pub address: Address,
    pub index: u32,
    pub join_date: u64,
    pub total_contributed: i128,
    pub payouts_received: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Contribution {
    pub member: Address,
    pub group_id: u64,
    pub cycle: u32,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct CycleInfo {
    pub group_id: u64,
    pub cycle_number: u32,
    pub total_pot: i128,
    pub current_recipient_index: u32,
    pub contributions_received: u32,
    pub expected_contributions: u32,
    pub is_fully_funded: bool,
    pub cycle_started_at: u64,
}

// ==================== CONTRACT IMPLEMENTATION ====================

#[contractimpl]
impl PaluwaganNiJuan {
    /// Initialize a new Paluwagan group
    pub fn initialize(
        env: Env,
        organizer: Address,
        token_contract: Address,
        member_count: u32,
        contribution_amount: i128,
        frequency_days: u32,
    ) -> u64 {
        organizer.require_auth();

        if member_count < 2 || member_count > 100 {
            panic!("Member count must be between 2 and 100");
        }
        if contribution_amount <= 0 {
            panic!("Contribution amount must be positive");
        }
        if frequency_days == 0 {
            panic!("Frequency must be at least 1 day");
        }

        let mut next_id: u64 = env
            .storage()
            .persistent()
            .get(&NEXT_GROUP_ID_KEY)
            .unwrap_or(1);

        let group_id = next_id;
        next_id += 1;

        let group = PaluwagGroup {
            group_id,
            organizer: organizer.clone(),
            token_contract,
            member_count,
            contribution_amount,
            frequency_days,
            status: GroupStatus::Active,
            created_at: env.ledger().timestamp(),
            current_cycle: 1,
            current_recipient_index: 0,
            contributions_this_cycle: 0,
        };

        let mut groups: Vec<PaluwagGroup> = env
            .storage()
            .persistent()
            .get(&GROUPS_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        groups.push_back(group);

        env.storage().persistent().set(&GROUPS_KEY, &groups);
        env.storage()
            .persistent()
            .set(&NEXT_GROUP_ID_KEY, &next_id);

        let empty_members: Vec<Member> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&(MEMBERS_KEY, group_id), &empty_members);

        group_id
    }

    /// Add a member to the Paluwagan group (organizer only)
    pub fn add_member(env: Env, group_id: u64, member_address: Address) -> bool {
        let mut groups: Vec<PaluwagGroup> = env
            .storage()
            .persistent()
            .get(&GROUPS_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        let mut group: Option<PaluwagGroup> = None;

        for i in 0..groups.len() {
            let g = groups.get_unchecked(i);
            if g.group_id == group_id {
                g.organizer.require_auth();
                group = Some(g.clone());
                break;
            }
        }

        let unwrapped_group = group.unwrap_or_else(|| panic!("Group not found"));

        let mut members: Vec<Member> = env
            .storage()
            .persistent()
            .get(&(MEMBERS_KEY, group_id))
            .unwrap_or_else(|| Vec::new(&env));

        for i in 0..members.len() {
            if members.get_unchecked(i).address == member_address {
                panic!("Member already exists in group");
            }
        }

        // Fix: compare u32 with u32
        if members.len() as u32 >= unwrapped_group.member_count {
            panic!("Group is at full capacity");
        }

        let new_member = Member {
            address: member_address,
            index: members.len() as u32,
            join_date: env.ledger().timestamp(),
            total_contributed: 0,
            payouts_received: 0,
        };

        members.push_back(new_member);
        env.storage()
            .persistent()
            .set(&(MEMBERS_KEY, group_id), &members);

        true
    }

    /// Member contributes their payment for the current cycle
    pub fn contribute(env: Env, group_id: u64, member: Address, amount: i128) -> bool {
        member.require_auth();

        let mut groups: Vec<PaluwagGroup> = env
            .storage()
            .persistent()
            .get(&GROUPS_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        // Fix: store index as usize for Rust indexing
        let mut group_idx: Option<usize> = None;
        let mut group: Option<PaluwagGroup> = None;

        for i in 0..groups.len() {
            let g = groups.get_unchecked(i);
            if g.group_id == group_id {
                group = Some(g.clone());
                group_idx = Some(i as usize); // Fix: cast u32 → usize
                break;
            }
        }

        let mut unwrapped_group = group.unwrap_or_else(|| panic!("Group not found"));

        if unwrapped_group.status != GroupStatus::Active {
            panic!("Group is not active");
        }

        if amount != unwrapped_group.contribution_amount {
            panic!("Amount does not match group contribution");
        }

        let mut members: Vec<Member> = env
            .storage()
            .persistent()
            .get(&(MEMBERS_KEY, group_id))
            .unwrap_or_else(|| Vec::new(&env));

        // Fix: store index as usize for Rust indexing
        let mut member_idx: Option<usize> = None;
        for i in 0..members.len() {
            if members.get_unchecked(i).address == member {
                member_idx = Some(i as usize); // Fix: cast u32 → usize
                break;
            }
        }

        if member_idx.is_none() {
            panic!("Member not found in group");
        }

        let contributions: Vec<Contribution> = env
            .storage()
            .persistent()
            .get(&(CONTRIBUTIONS_KEY, group_id, unwrapped_group.current_cycle))
            .unwrap_or_else(|| Vec::new(&env));

        for i in 0..contributions.len() {
            if contributions.get_unchecked(i).member == member {
                panic!("Member already contributed this cycle");
            }
        }

        let token_client = token::Client::new(&env, &unwrapped_group.token_contract);
        token_client.transfer(&member, &env.current_contract_address(), &amount);

        let contribution = Contribution {
            member: member.clone(),
            group_id,
            cycle: unwrapped_group.current_cycle,
            amount,
            timestamp: env.ledger().timestamp(),
        };

        let mut contributions = contributions;
        contributions.push_back(contribution);

        env.storage().persistent().set(
            &(CONTRIBUTIONS_KEY, group_id, unwrapped_group.current_cycle),
            &contributions,
        );

        // Fix: cast usize → u32 for Soroban Vec
        let idx = member_idx.unwrap();
        let mut unwrapped_member = members.get_unchecked(idx as u32).clone();
        unwrapped_member.total_contributed += amount;
        members.set(idx as u32, unwrapped_member); // Fix: cast usize → u32

        env.storage()
            .persistent()
            .set(&(MEMBERS_KEY, group_id), &members);

        unwrapped_group.contributions_this_cycle += 1;

        if let Some(idx) = group_idx {
            groups.set(idx as u32, unwrapped_group); // Fix: cast usize → u32
            env.storage().persistent().set(&GROUPS_KEY, &groups);
        }

        true
    }

    /// Claim payout when cycle is fully funded (current recipient only)
    pub fn claim_payout(env: Env, group_id: u64) -> i128 {
        let mut groups: Vec<PaluwagGroup> = env
            .storage()
            .persistent()
            .get(&GROUPS_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        let mut group_idx: Option<usize> = None;
        let mut group: Option<PaluwagGroup> = None;

        for i in 0..groups.len() {
            let g = groups.get_unchecked(i);
            if g.group_id == group_id {
                group = Some(g.clone());
                group_idx = Some(i as usize); // Fix: cast u32 → usize
                break;
            }
        }

        let mut unwrapped_group = group.unwrap_or_else(|| panic!("Group not found"));

        if unwrapped_group.contributions_this_cycle < unwrapped_group.member_count {
            panic!("Cycle not fully funded yet");
        }

        let members: Vec<Member> = env
            .storage()
            .persistent()
            .get(&(MEMBERS_KEY, group_id))
            .unwrap_or_else(|| Vec::new(&env));

        if members.len() == 0 {
            panic!("No members in group");
        }

        // Fix: current_recipient_index is already u32, no cast needed
        let recipient = members.get_unchecked(unwrapped_group.current_recipient_index);
        recipient.address.require_auth();

        let pot = unwrapped_group.contribution_amount * (unwrapped_group.member_count as i128);

        let token_client = token::Client::new(&env, &unwrapped_group.token_contract);
        token_client.transfer(
            &env.current_contract_address(),
            &recipient.address,
            &pot,
        );

        let mut updated_members = members;
        let mut updated_recipient = recipient.clone();
        updated_recipient.payouts_received += 1;
        // Fix: current_recipient_index is already u32
        updated_members.set(
            unwrapped_group.current_recipient_index,
            updated_recipient,
        );
        env.storage()
            .persistent()
            .set(&(MEMBERS_KEY, group_id), &updated_members);

        unwrapped_group.current_recipient_index =
            (unwrapped_group.current_recipient_index + 1) % unwrapped_group.member_count;

        if unwrapped_group.current_recipient_index == 0 {
            unwrapped_group.current_cycle += 1;
        }

        unwrapped_group.contributions_this_cycle = 0;

        if unwrapped_group.current_recipient_index == 0 && unwrapped_group.current_cycle > 1 {
            unwrapped_group.status = GroupStatus::Completed;
        }

        if let Some(idx) = group_idx {
            groups.set(idx as u32, unwrapped_group); // Fix: cast usize → u32
            env.storage().persistent().set(&GROUPS_KEY, &groups);
        }

        pot
    }

    /// Read-only: Get current cycle information
    pub fn get_cycle_info(env: Env, group_id: u64) -> CycleInfo {
        let groups: Vec<PaluwagGroup> = env
            .storage()
            .persistent()
            .get(&GROUPS_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        let mut group: Option<PaluwagGroup> = None;
        for i in 0..groups.len() {
            let g = groups.get_unchecked(i);
            if g.group_id == group_id {
                group = Some(g.clone());
                break;
            }
        }

        let unwrapped_group = group.unwrap_or_else(|| panic!("Group not found"));

        let pot = unwrapped_group.contribution_amount * (unwrapped_group.member_count as i128);
        let is_fully_funded =
            unwrapped_group.contributions_this_cycle >= unwrapped_group.member_count;

        CycleInfo {
            group_id,
            cycle_number: unwrapped_group.current_cycle,
            total_pot: pot,
            current_recipient_index: unwrapped_group.current_recipient_index,
            contributions_received: unwrapped_group.contributions_this_cycle,
            expected_contributions: unwrapped_group.member_count,
            is_fully_funded,
            cycle_started_at: unwrapped_group.created_at,
        }
    }

    /// Read-only: Get group details
    pub fn get_group(env: Env, group_id: u64) -> PaluwagGroup {
        let groups: Vec<PaluwagGroup> = env
            .storage()
            .persistent()
            .get(&GROUPS_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        for i in 0..groups.len() {
            let g = groups.get_unchecked(i);
            if g.group_id == group_id {
                return g.clone();
            }
        }

        panic!("Group not found");
    }

    /// Read-only: Get all members of a group
    pub fn get_members(env: Env, group_id: u64) -> Vec<Member> {
        env.storage()
            .persistent()
            .get(&(MEMBERS_KEY, group_id))
            .unwrap_or_else(|| Vec::new(&env))
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Ledger;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_initialize_group_happy_path() {
        let env = Env::default();
        let contract = PaluwaganNiJuan;

        let organizer = Address::generate(&env);
        let token = Address::generate(&env);

        let group_id = contract.initialize(
            env.clone(),
            organizer.clone(),
            token.clone(),
            5,
            1_000_000,
            7,
        );

        assert_eq!(group_id, 1, "First group should have ID 1");

        let group = contract.get_group(env.clone(), group_id);
        assert_eq!(group.member_count, 5);
        assert_eq!(group.contribution_amount, 1_000_000);
        assert_eq!(group.status, GroupStatus::Active);
    }

    #[test]
    fn test_add_member_succeeds() {
        let env = Env::default();
        let contract = PaluwaganNiJuan;

        let organizer = Address::generate(&env);
        let token = Address::generate(&env);
        let member1 = Address::generate(&env);

        let group_id = contract.initialize(
            env.clone(),
            organizer.clone(),
            token.clone(),
            5,
            1_000_000,
            7,
        );

        let result = contract.add_member(env.clone(), group_id, member1.clone());
        assert!(result, "Member should be added successfully");

        let members = contract.get_members(env.clone(), group_id);
        assert_eq!(members.len(), 1, "Group should have 1 member");
        assert_eq!(members.get_unchecked(0).address, member1);
    }

    #[test]
    fn test_get_cycle_info_pot_calculation() {
        let env = Env::default();
        let contract = PaluwaganNiJuan;

        let organizer = Address::generate(&env);
        let token = Address::generate(&env);

        let group_id = contract.initialize(
            env.clone(),
            organizer.clone(),
            token.clone(),
            4,
            1_000_000,
            7,
        );

        let cycle_info = contract.get_cycle_info(env.clone(), group_id);
        assert_eq!(cycle_info.total_pot, 4_000_000, "Pot should be 4 × 1_000_000");
        assert_eq!(cycle_info.cycle_number, 1);
        assert_eq!(cycle_info.contributions_received, 0);
        assert_eq!(cycle_info.expected_contributions, 4);
        assert_eq!(cycle_info.is_fully_funded, false);
        assert_eq!(cycle_info.current_recipient_index, 0);
    }

    #[test]
    fn test_group_not_found_error() {
        let env = Env::default();
        let contract = PaluwaganNiJuan;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.get_group(env.clone(), 999);
        }));

        assert!(result.is_err(), "Should panic on group not found");
    }
}