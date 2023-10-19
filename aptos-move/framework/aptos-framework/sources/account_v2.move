module aptos_framework::account_v2 {
    use std::error;
    use std::signer;
    use aptos_framework::aptos_account::assert_account_exists;
    use aptos_framework::object;
    use aptos_framework::object::{ConstructorRef, ExtendRef, Object, is_object, TransferRef};

    friend aptos_framework::aptos_account;
    friend aptos_framework::resource_account;


    const EACCOUNT_ALREADY_EXISTS: u64 = 1;
    const ECANNOT_RESERVED_ADDRESS: u64 = 2;
    const EACCOUNT_ALREADY_USED: u64 = 3;
    const ESEQUENCE_NUMBER_TOO_BIG: u64 = 4;
    /// The native authenticator index has to be consistent with rust code.
    const AUTHENTICATION_RESERVED: u8 = 255;

    const MAX_U64: u128 = 18446744073709551615;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Account has key {
        sequence_number: u64,
        transfer_ref: TransferRef,
        extend_ref: ExtendRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct NativeAuthenticator has key, copy, store, drop {
        key: vector<u8>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct CustomizedAuthenticator has key, copy, store, drop {
        account_address: address,
        module_name: vector<u8>,
    }

    public fun gen_native_authenticator(key: vector<u8>): NativeAuthenticator {
        NativeAuthenticator {
            key,
        }
    }

    public fun gen_customized_authenticator(
        account_address: address,
        module_name: vector<u8>,
    ): CustomizedAuthenticator {
        CustomizedAuthenticator { account_address, module_name }
    }

    entry fun update_native_authenticator(
        account: &signer,
        key: vector<u8>,
    ) acquires CustomizedAuthenticator, NativeAuthenticator {
        update_native_authenticator_impl(account, NativeAuthenticator {
            key,
        });
    }

    public(friend) fun update_native_authenticator_impl(
        account: &signer,
        authenticator: NativeAuthenticator
    ) acquires CustomizedAuthenticator, NativeAuthenticator {
        let account_address = signer::address_of(account);
        assert_account_exists(account_address);
        if (exists<CustomizedAuthenticator>(account_address)) {
            move_from<CustomizedAuthenticator>(account_address);
        };
        if (exists<NativeAuthenticator>(account_address)) {
            let current = borrow_global_mut<NativeAuthenticator>(account_address);
            if (*current != authenticator) {
                *current = authenticator;
            }
        } else {
            move_to(account, authenticator)
        }
    }

    public(friend) fun update_customized_authenticator_impl(
        account: &signer,
        authenticator: CustomizedAuthenticator
    ) acquires CustomizedAuthenticator, NativeAuthenticator {
        let account_address = signer::address_of(account);
        assert_account_exists(account_address);
        if (exists<NativeAuthenticator>(account_address)) {
            move_from<NativeAuthenticator>(account_address);
        };
        if (exists<CustomizedAuthenticator>(account_address)) {
            let current = borrow_global_mut<CustomizedAuthenticator>(account_address);
            if (*current != authenticator) {
                *current = authenticator;
            }
        } else {
            move_to(account, authenticator)
        }
    }

    public fun generate_signer_from_owner(owner: &signer, account: Object<Account>): signer acquires Account {
        assert_account_exists(object::object_address(&account));
        assert!(object::is_owner(account, signer::address_of(owner)), 0);
        let account = borrow_global<Account>(object::object_address(&account));
        object::generate_signer_for_extending(&account.extend_ref)
    }

    /// Transfer ownership of the account.
    public fun transfer(account: &signer, owner: address) acquires Account {
        let account_address = signer::address_of(account);
        assert_account_exists(account_address);
        let current_owner = object::owner(object::address_to_object<Account>(account_address));
        if (current_owner == owner) {
            return;
        };
        let linear_transfer_ref = object::generate_linear_transfer_ref(
            &borrow_global<Account>(account_address).transfer_ref
        );
        object::transfer_with_ref(linear_transfer_ref, owner);
    }

    /// A utility function to change the ownership of the account to itself.
    public inline fun make_self_owned(account: &signer) {
        aptos_framework::account_v2::transfer(account, std::signer::address_of(account));
    }

    public fun create_resource_account(source: &signer, seed: vector<u8>): signer acquires Account {
        let resource_addr = object::create_object_address(&signer::address_of(source), seed);
        assert!(!exists_at(resource_addr), error::invalid_state(EACCOUNT_ALREADY_USED));
        let self = create_account_unchecked(resource_addr);
        transfer(&self, signer::address_of(source));
        self
    }

    /// Publishes a new `Account` resource under `new_address`. A ConstructorRef representing `new_address`
    /// is returned. This way, the caller of this function can publish additional resources under
    /// `new_address`.
    public(friend) fun create_account(new_address: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!is_object(new_address), error::already_exists(EACCOUNT_ALREADY_EXISTS));
        assert!(!exists<Account>(new_address), error::already_exists(EACCOUNT_ALREADY_EXISTS));

        // NOTE: @core_resources gets created via a `create_account` call, so we do not include it below.
        assert!(
            new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
            error::invalid_argument(ECANNOT_RESERVED_ADDRESS)
        );
        create_account_unchecked(new_address)
    }


    fun create_account_unchecked(new_address: address): signer {
        let new_account_cref = &object::create_object_at_address(new_address);
        let new_account = &object::generate_signer(new_account_cref);
        move_to(
            new_account,
            Account {
                sequence_number: 0,
                transfer_ref: object::generate_transfer_ref(new_account_cref),
                extend_ref: object::generate_extend_ref(new_account_cref)
            }
        );
        object::generate_signer(new_account_cref)
    }

    #[view]
    public fun exists_at(addr: address): bool {
        is_object(addr) && exists<Account>(addr)
    }

    #[view]
    public fun use_native_authenticator(addr: address): bool {
        exists<NativeAuthenticator>(addr)
    }

    #[view]
    public fun use_customized_authenticator(addr: address): bool {
        exists<CustomizedAuthenticator>(addr)
    }

    #[view]
    public fun get_sequence_number(addr: address): u64 acquires Account {
        borrow_global<Account>(addr).sequence_number
    }

    #[view]
    public fun get_native_authentication_key(addr: address): vector<u8> acquires NativeAuthenticator {
        assert!(use_native_authenticator(addr), 0);
        borrow_global<NativeAuthenticator>(addr).key
    }

    // Only called by transaction_validation.move in apilogue for sequential transactions.
    public(friend) fun increment_sequence_number(addr: address) acquires Account {
        let sequence_number = &mut borrow_global_mut<Account>(addr).sequence_number;

        assert!(
            (*sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)
        );
        *sequence_number = *sequence_number + 1;
    }

    #[test_only]
    public fun create_account_for_test(new_address: address): signer {
        create_account_unchecked(new_address)
    }
}
