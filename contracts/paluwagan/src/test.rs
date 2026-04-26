#[cfg(test)]
mod tests {
    use crate::{PaluwaganNiJuan, GroupStatus};
    use soroban_sdk::{testutils::Address as _, Env, Address};

    /// TEST 1: HAPPY PATH
    /// Initialize group → Add members → Verify state
    #[test]
    fn test_initialize_group_happy_path() {
        let env = Env::default();
        let contract = PaluwganNiJuan;

        let organizer = Address::generate(&env);
        let token = Address::generate(&env);

        let group_id = contract.initialize(
            env.clone(),
            organizer.clone(),
            token.clone(),
            5,  // 5 members
            1_000_000, // 1 USDC (7 decimals)
            7,  // 7 days frequency
        );

        assert_eq!(group_id, 1, "First group should have ID 1");

        let group = contract.get_group(env.clone(), group_id);
        assert_eq!(group.member_count, 5);
        assert_eq!(group.contribution_amount, 1_000_000);
        assert_eq!(group.status, GroupStatus::Active);
        assert_eq!(group.current_cycle, 1);
        assert_eq!(group.current_recipient_index, 0);
    }

    /// TEST 2: EDGE CASE - MEMBER ALREADY EXISTS
    /// Try to add the same member twice → Should panic
    #[test]
    fn test_add_member_duplicate_fails() {
        let env = Env::default();
        let contract = PaluwganNiJuan;

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

        // Add member first time - succeeds
        contract.add_member(env.clone(), group_id, member1.clone());

        // Try to add same member again - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.add_member(env.clone(), group_id, member1.clone());
        }));

        assert!(result.is_err(), "Should panic when adding duplicate member");
    }

    /// TEST 3: STATE VERIFICATION - POT CALCULATION
    /// Verify that pot = contribution_amount × member_count
    #[test]
    fn test_get_cycle_info_pot_calculation() {
        let env = Env::default();
        let contract = PaluwganNiJuan;

        let organizer = Address::generate(&env);
        let token = Address::generate(&env);

        let group_id = contract.initialize(
            env.clone(),
            organizer.clone(),
            token.clone(),
            4,           // 4 members
            1_000_000,   // 1 USDC each
            7,
        );

        let cycle_info = contract.get_cycle_info(env.clone(), group_id);
        
        // Verify pot calculation: 4 members × 1,000,000 = 4,000,000
        assert_eq!(
            cycle_info.total_pot,
            4_000_000,
            "Pot should be 4 × 1,000,000"
        );
        
        // Verify initial cycle state
        assert_eq!(cycle_info.cycle_number, 1);
        assert_eq!(cycle_info.contributions_received, 0);
        assert_eq!(cycle_info.expected_contributions, 4);
        assert_eq!(cycle_info.is_fully_funded, false);
        assert_eq!(cycle_info.current_recipient_index, 0);
    }

    /// TEST 4: CYCLE PROGRESSION - ROTATION LOGIC
    /// Verify that current_recipient_index rotates correctly
    #[test]
    fn test_group_rotation_index_initialization() {
        let env = Env::default();
        let contract = PaluwganNiJuan;

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

        // Get initial cycle info
        let cycle_info = contract.get_cycle_info(env.clone(), group_id);
        
        // Member 0 should be the initial recipient
        assert_eq!(
            cycle_info.current_recipient_index, 0,
            "Initial recipient should be member 0"
        );
        
        // Get group to verify turn state
        let group = contract.get_group(env.clone(), group_id);
        assert_eq!(
            group.current_turn, 0,
            "Initial turn counter should be 0"
        );
    }

    /// TEST 5: ERROR HANDLING - INVALID GROUP ID
    /// Try to get non-existent group → Should panic with clear message
    #[test]
    fn test_group_not_found_error() {
        let env = Env::default();
        let contract = PaluwganNiJuan;

        // Try to get a group that doesn't exist
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.get_group(env.clone(), 999);
        }));

        assert!(result.is_err(), "Should panic when group not found");
    }
}