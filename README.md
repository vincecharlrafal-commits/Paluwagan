Paluwagan ni Juan - Design & Architecture
High-Level Architecture
Backend: On-Chain Smart Contract
```
Stellar Blockchain (Testnet)
    ↓
Soroban Smart Contract (Rust)
    ├── Escrow Logic: Fund locking, rotation
    ├── State Management: Groups, members, contributions
    └── Token Integration: USDC transfers
    ↓
Persistent Storage (Ledger)
    ├── Groups index
    ├── Member roster
    ├── Contribution records
    └── Cycle state
```
Key Principle: No backend server. The blockchain is the backend.
---
Payout Sequence Logic
Cycle Definition
A cycle is one complete round where:
All N members contribute their fixed amount
The current recipient receives the full pot
Rotation advances to the next member
Detailed State Diagram
```
┌─────────────────────────────────────────────────────────────┐
│ CYCLE 1: Member 0 is recipient                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Week 1:                                                     │
│  ├─ Member 0 contributes 10 USDC (contract locks)          │
│  ├─ Member 1 contributes 10 USDC (contract locks)          │
│  └─ Member 2 contributes 10 USDC (contract locks)          │
│     [Total pot: 30 USDC, all members confirmed]            │
│                                                               │
│  ├─ Member 0 calls claim_payout()                          │
│  ├─ Contract releases 30 USDC → Member 0 wallet           │
│  ├─ Member 0.payouts_received = 1                         │
│  └─ Rotation: current_recipient_index = 1                 │
│                                                               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ CYCLE 1: Member 1 is recipient                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Week 2:                                                     │
│  ├─ contributions_this_cycle RESET to 0                    │
│  ├─ Member 0 contributes 10 USDC                           │
│  ├─ Member 1 contributes 10 USDC                           │
│  └─ Member 2 contributes 10 USDC                           │
│                                                               │
│  ├─ Member 1 calls claim_payout()                          │
│  ├─ Contract releases 30 USDC → Member 1 wallet           │
│  ├─ Member 1.payouts_received = 1                         │
│  └─ Rotation: current_recipient_index = 2                 │
│                                                               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ CYCLE 1: Member 2 is recipient (FINAL)                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Week 3:                                                     │
│  ├─ Member 0 contributes 10 USDC                           │
│  ├─ Member 1 contributes 10 USDC                           │
│  └─ Member 2 contributes 10 USDC                           │
│                                                               │
│  ├─ Member 2 calls claim_payout()                          │
│  ├─ Contract releases 30 USDC → Member 2 wallet           │
│  ├─ Member 2.payouts_received = 1                         │
│  ├─ Rotation: current_recipient_index = (2 + 1) % 3 = 0   │
│  ├─ Wrapped to 0 → current_cycle incremented to 2         │
│  └─ Group status: Completed (all members paid)             │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```
Payout Algorithm (Pseudocode)
```rust
fn claim_payout(group_id, caller_address) {
    group = get_group(group_id)
    
    // 1. Verify cycle is fully funded
    assert(group.contributions_this_cycle == group.member_count)
    
    // 2. Verify caller is current recipient
    recipient = group.members[group.current_recipient_index]
    assert(caller_address == recipient.address)
    
    // 3. Calculate and transfer pot
    pot = group.contribution_amount × group.member_count
    transfer(CONTRACT, recipient, pot)
    
    // 4. Update recipient metrics
    recipient.payouts_received += 1
    
    // 5. Rotate to next recipient
    group.current_recipient_index += 1
    
    // 6. Handle wrap-around
    if group.current_recipient_index >= group.member_count {
        group.current_recipient_index = 0
        group.current_cycle += 1
    }
    
    // 7. Reset contribution tracking
    group.contributions_this_cycle = 0
    
    // 8. Check if all members have been paid
    if all_members_paid(group) {
        group.status = Completed
    }
    
    // 9. Persist state
    save_group(group)
    
    return pot
}
```
---
Storage Model & Persistence
Group Registry
```
GROUPS_KEY → [
    { id: 1, organizer: ADDR_A, member_count: 20, ... },
    { id: 2, organizer: ADDR_B, member_count: 5, ... },
    { id: 3, organizer: ADDR_C, member_count: 10, ... },
]
```
Member Roster (per group)
```
(MEMBERS_KEY, group_id=1) → [
    { address: MEMBER_0, index: 0, payouts_received: 1, total_contributed: 30_000_000 },
    { address: MEMBER_1, index: 1, payouts_received: 0, total_contributed: 20_000_000 },
    { address: MEMBER_2, index: 2, payouts_received: 0, total_contributed: 10_000_000 },
]
```
Contribution Records (per group, per cycle)
```
(CONTRIBUTIONS_KEY, group_id=1, cycle=1) → [
    { member: MEMBER_0, amount: 10_000_000, timestamp: 1234567890 },
    { member: MEMBER_1, amount: 10_000_000, timestamp: 1234567891 },
    { member: MEMBER_2, amount: 10_000_000, timestamp: 1234567892 },
]
```
Group Sequence Counter
```
NEXT_GROUP_ID_KEY → 4  (next group will be ID 4)
```
---
Key Design Decisions
Decision 1: On-Chain Everything
Rationale: No backend server = no single point of failure, no central authority.  
Trade-off: Storage costs on Stellar ledger (acceptable for small group sizes).
Decision 2: Immutable Contribution Records
Rationale: Blockchain provides permanent audit trail.  
Benefit: Members can prove their savings history for credit applications.
Decision 3: Sequential Rotation (No Bidding)
Rationale: Simple, predictable, fair. Reduces complexity.  
Trade-off: Members receive payout on a fixed schedule, no flexibility.
Decision 4: All Members Must Contribute Each Cycle
Rationale: Ensures every member knows the exact pot they'll receive.  
Trade-off: One missing contribution blocks the entire cycle.  
Mitigation: Frontend can send reminders; groups can decide grace period off-chain.
Decision 5: Caller-Driven Payout Trigger
Rationale: Recipient decides when to claim (no relayer needed).  
Benefit: No trust in third party to release funds.  
Trade-off: Recipient must be online and willing to claim.
---
Security Model
Threat 1: Early Recipient Disappears
Traditional: Member gets ₱5,000 pot, vanishes. Later members lose.  
Paluwagan ni Juan: Funds locked in contract until all members contribute. Early recipient gets their turn, not before.
Threat 2: Organizer Steals Funds
Traditional: Organizer holds the pot; embezzles.  
Paluwagan ni Juan: Contract holds funds. Only the designated recipient can claim.
Threat 3: Dispute Over Contributions
Traditional: Notebook vs. word-of-mouth. No proof.  
Paluwagan ni Juan: Stellar blockchain is the receipt. Immutable and auditable.
Threat 4: False Payout Claim
Traditional: Member claims they're the recipient; organizer confused.  
Paluwagan ni Juan: Contract enforces rotation index. Only member at `current_recipient_index` can claim.
---
Scalability Considerations
Current Limits
Members per group: 2–100 (reasonable for community groups)
Contribution amount: Any i128 value (up to ~9 × 10^18)
Frequency: Daily to yearly (any positive integer days)
Groups: Unlimited (each group_id is unique)
Storage Cost per Group
```
Group struct: ~200 bytes
Members: 100 bytes per member × N members
Contributions: ~50 bytes per contribution × N members × cycles completed
Total for 20-member group after 3 cycles: ~10 KB
```
Negligible compared to typical Stellar ledger usage.
Performance
Initialize group: O(1)
Add member: O(N) where N = current members
Contribute: O(N) to check duplicate
Claim payout: O(N) to find recipient
Get cycle info: O(1)
All acceptable for group sizes ≤100.
---
Future Enhancements
Phase 2: Flexible Payout
Allow bidding or negotiated turn order instead of strict rotation.
Phase 3: Partial Contributions
Handle member absences with partial pot carryover to next cycle.
Phase 4: Multi-Currency Support
Support USDC, XLM, PHP stablecoins simultaneously.
Phase 5: Governance
Allow groups to vote on modifications (contribution amount, member additions).
Phase 6: Lending Layer
Composable: groups can borrow against future payouts at low rates.
---
Testing Strategy
Unit Tests (In-Contract)
```
Test 1: Happy Path
  Initialize → Add 5 members → All contribute → First claims → Advances turn ✓

Test 2: Edge Case (Duplicate Contribution)
  Member contributes twice in same cycle → Panic ✓

Test 3: State Verification
  After payout, verify pot = 5 × contribution_amount ✓

Test 4: Cycle Progression
  After member 4 claims, verify current_recipient_index = 0, current_cycle = 2 ✓

Test 5: Error Handling
  Invalid group_id → Panic with clear message ✓
```
Integration Tests (Manual CLI)
```
1. Deploy to testnet
2. Fund test accounts with USDC
3. Run full 3-member paluwagan cycle end-to-end
4. Verify funds in recipient's wallet
5. Verify Stellar Expert shows correct state
```
---
Deployment & Operations
Pre-Deployment Checklist
[ ] All tests passing
[ ] No compiler warnings
[ ] Contract builds as WASM
[ ] README complete with examples
[ ] Testnet USDC contract ID known
Deployment Steps
```bash
1. soroban contract build
2. soroban contract deploy --wasm ... --source deployer --network testnet
3. Save contract ID
4. Update README with Contract ID
5. Test via CLI
```
Post-Deployment Monitoring
Monitor testnet for transaction patterns
Track group creation count
Sample random groups for data integrity
Collect feedback from real user groups
---
Conclusion
Paluwagan ni Juan is a minimal, trustless ROSCA that solves the core Filipino problem: how to save together without trusting a middleman. By moving the escrow to Stellar, we give millions of Filipinos a chance to save securely.
The smart contract enforces the rules. The blockchain is the receipt. And the payouts are inevitable.