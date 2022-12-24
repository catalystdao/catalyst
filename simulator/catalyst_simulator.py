from dataclasses import dataclass
import time
from typing import Dict, Generic, List, Tuple, Type

from solana.publickey import PublicKey as SolanaPublicKey
from integer import Int256, TInt, TUint, Uint256, is_real_int

from swap_calculation_helpers import ONE_X64, calc_asset_amount_for_pool_tokens_f, calc_asset_amount_for_pool_tokens_i, calc_in_liquidity_swap_f, calc_in_liquidity_swap_i, calc_out_liquidity_swap_f, calc_out_liquidity_swap_i_x64, full_swap_f, full_swap_i, in_swap_f, in_swap_i, out_swap_f, out_swap_i_x64

from fixed_point_math import inv_pow2_x64, mul_x64, pow_x64

#! TODOs
# - Use fixed point or floating point numbers? ==> Use floating, set errors threshold ==> avoid flawed implementations in both test and production logic

AssetId      = int | SolanaPublicKey
UserId       = int
SourceSwapId = int


@dataclass
class CatalystSimulatorSnapshot(Generic[TUint, TInt]):

    uint_type : Type[TUint]
    int_type  : Type[TInt]

    DECAY_RATE : int

    amplification_i_x64 : Uint256 | None
    amplification_f     : float | None

    assets_balances_i          : Dict[AssetId, TUint]
    assets_eq_balances_i       : Dict[AssetId, TUint]
    assets_weights_i           : Dict[AssetId, TUint]
    pool_tokens_supply_i       : TUint
    pool_tokens_distribution_i : Dict[UserId, TUint]    # Keep track of the depositors pool token balances
    escrowed_assets_i          : Dict[AssetId, TUint]
    escrows_i                  : Dict[SourceSwapId, Tuple[AssetId, TUint]]

    max_units_inflow_i_x64               : Uint256
    current_units_inflow_i_x64           : Uint256
    current_units_inflow_timestamp_i     : TUint      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'
    current_liquidity_inflow_i           : TUint
    current_liquidity_inflow_timestamp_i : TUint      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'


    assets_balances_f          : Dict[AssetId, float]
    assets_eq_balances_f       : Dict[AssetId, float]
    assets_weights_f           : Dict[AssetId, float]
    pool_tokens_supply_f       : float
    pool_tokens_distribution_f : Dict[UserId, float]    # Keep track of the depositors pool token balances
    escrowed_assets_f          : Dict[AssetId, float]
    escrows_f                  : Dict[SourceSwapId, Tuple[AssetId, float]]

    max_units_inflow_f                   : float
    current_units_inflow_f               : float
    current_units_inflow_timestamp_f     : float      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'
    current_liquidity_inflow_f           : float
    current_liquidity_inflow_timestamp_f : float      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'



class CatalystSimulator(Generic[TUint, TInt]):

    uint_type : Type[TUint]
    int_type  : Type[TInt]

    DECAY_RATE : int = 60*60*24

    amplification_i_x64 : Uint256 | None
    amplification_f     : float | None

    assets_balances_i          : Dict[AssetId, TUint]
    assets_eq_balances_i       : Dict[AssetId, TUint]
    assets_weights_i           : Dict[AssetId, TUint]
    pool_tokens_supply_i       : TUint
    pool_tokens_distribution_i : Dict[UserId, TUint]    # Keep track of the depositors pool token balances
    escrowed_assets_i          : Dict[AssetId, TUint]
    escrows_i                  : Dict[SourceSwapId, Tuple[AssetId, TUint]]

    max_units_inflow_i_x64               : Uint256
    current_units_inflow_i_x64           : Uint256
    current_units_inflow_timestamp_i     : TUint      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'
    current_liquidity_inflow_i           : TUint
    current_liquidity_inflow_timestamp_i : TUint      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'


    assets_balances_f          : Dict[AssetId, float]
    assets_eq_balances_f       : Dict[AssetId, float]
    assets_weights_f           : Dict[AssetId, float]
    pool_tokens_supply_f       : float
    pool_tokens_distribution_f : Dict[UserId, float]    # Keep track of the depositors pool token balances
    escrowed_assets_f          : Dict[AssetId, float]
    escrows_f                  : Dict[SourceSwapId, Tuple[AssetId, float]]

    max_units_inflow_f                   : float
    current_units_inflow_f               : float
    current_units_inflow_timestamp_f     : float      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'
    current_liquidity_inflow_f           : float
    current_liquidity_inflow_timestamp_f : float      # TODO do we want a specific '_i' variable? Or use common varaible for both '_i' and '_f'


    def __init__(
        self,
        amplification: TUint | int | None,
        assets: List[AssetId],
        assets_weights: List[int],
        init_assets_balances: List[int],
        depositor: UserId,
        uint_type: Type[TUint] = Uint256,
        int_type: Type[TInt] = Int256
    ):
        self.amplification_f     = None if amplification is None else 1 / (amplification.value if is_real_int(amplification) else amplification)     #type: ignore
        self.amplification_i_x64 = None if amplification is None else Uint256(int(self.amplification_f*2**64))

        self.uint_type = uint_type
        self.int_type = int_type


        self.assets_balances_i          = {}
        self.assets_eq_balances_i       = {}
        self.assets_weights_i           = {}
        self.pool_tokens_supply_i       = self.uint_type(0)
        self.pool_tokens_distribution_i = {}
        self.escrowed_assets_i          = {}
        self.escrows_i                  = {}

        self.max_units_inflow_i_x64               = Uint256(0)
        self.current_units_inflow_i_x64           = Uint256(0)
        self.current_units_inflow_timestamp_i     = self.uint_type(0)
        self.current_liquidity_inflow_i           = self.uint_type(0)
        self.current_liquidity_inflow_timestamp_i = self.uint_type(0)
        self.unit_tracker_i_x64                   = self.int_type(0)
        self.units_inflow_amplification_i_x64     = None


        self.assets_balances_f          = {}
        self.assets_eq_balances_f       = {}
        self.assets_weights_f           = {}
        self.pool_tokens_supply_f       = 0
        self.pool_tokens_distribution_f = {}
        self.escrowed_assets_f          = {}
        self.escrows_f                  = {}

        self.max_units_inflow_f                   = 0
        self.current_units_inflow_f               = 0
        self.current_units_inflow_timestamp_f     = 0
        self.current_liquidity_inflow_f           = 0
        self.current_liquidity_inflow_timestamp_f = 0
        self.unit_tracker_f                       = 0
        self.units_inflow_amplification_f         = None

        one_minus_amp_i_x64 = None if self.amplification_i_x64 is None else ONE_X64 - self.amplification_i_x64
        one_minus_amp_f     = None if self.amplification_f is None     else 1 - self.amplification_f

        for i, asset in enumerate(assets):
            self.assets_balances_i[asset]    = self.uint_type(init_assets_balances[i])
            self.assets_eq_balances_i[asset] = self.uint_type(init_assets_balances[i])
            self.assets_weights_i[asset]     = self.uint_type(assets_weights[i])
            self.escrowed_assets_i[asset]    = self.uint_type(0)

            self.assets_balances_f[asset]    = init_assets_balances[i]
            self.assets_eq_balances_f[asset] = init_assets_balances[i]
            self.assets_weights_f[asset]     = assets_weights[i]
            self.escrowed_assets_f[asset]    = 0

            # Set max_units_inflow
            if amplification is None:
                self.max_units_inflow_i_x64 += assets_weights[i] << 64
                self.max_units_inflow_f     += assets_weights[i]
            else:
                assert one_minus_amp_i_x64 is not None  # Make compiler happy
                assert one_minus_amp_f is not None      # Make compiler happy

                self.max_units_inflow_i_x64 += Uint256(assets_weights[i]) * pow_x64(
                    Uint256(init_assets_balances[i]) << 64, one_minus_amp_i_x64
                )

                self.max_units_inflow_f += assets_weights[i] * init_assets_balances[i]**one_minus_amp_f

        if one_minus_amp_i_x64 is not None:
            assert one_minus_amp_f is not None      # Make compiler happy
            self.units_inflow_amplification_i_x64 = ONE_X64 - inv_pow2_x64(one_minus_amp_i_x64)
            self.units_inflow_amplification_f     = 1 - 2**(-one_minus_amp_f)

            self.max_units_inflow_i_x64 = mul_x64(self.units_inflow_amplification_i_x64, self.max_units_inflow_i_x64)
            self.max_units_inflow_f     = self.units_inflow_amplification_f * self.max_units_inflow_f

        init_pool_tokens_amount = 1000000   #TODO set value as argument?

        self.pool_tokens_distribution_i[depositor] = self.uint_type(init_pool_tokens_amount)
        self.pool_tokens_supply_i                  = self.uint_type(init_pool_tokens_amount)

        self.pool_tokens_distribution_f[depositor] = init_pool_tokens_amount
        self.pool_tokens_supply_f                  = init_pool_tokens_amount

    @property
    def assets(self) -> List[AssetId]:
        return list(self.assets_balances_i.keys())



    # Deposit *******************************************************************************************************************

    def deposit(
        self,
        pool_tokens_amount : TUint | int,
        user               : UserId,
        timestamp          : int | None = None
    ) -> Dict[AssetId, TUint]:      # TODO create class to hold both integer and float results + accuracy stats?

        timestamp = timestamp or get_current_timestamp()

        pool_tokens_amount = self.uint_type(pool_tokens_amount)

        deposit_result_i = self._deposit_i(pool_tokens_amount, user, timestamp)
        self._deposit_f(pool_tokens_amount.value, user, timestamp)

        # TODO match flow timestamps _i and _f

        return deposit_result_i

    
    def _deposit_i( # TODO rename this to _deposit_i
        self,
        pool_tokens_amount : TUint,
        user               : UserId,
        timestamp          : TUint | int
    ) -> Dict[AssetId, TUint]:

        # Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
        # upwards by depositing changes the limit.
        self.update_liquidity_units_inflow_i(self.uint_type(0), self.uint_type(timestamp))

        deposited_amounts: Dict[AssetId, TUint] = {}

        for asset in self.assets:

            pool_tokens_for_asset = (self.assets_eq_balances_i[asset] * pool_tokens_amount) / self.pool_tokens_supply_i
            
            asset_deposit_balance = calc_asset_amount_for_pool_tokens_i(
                pool_token_balance = pool_tokens_for_asset,
                asset_balance      = self.assets_balances_i[asset],     # Escrowed tokens are NOT subtracted from the total balance => deposits should return less
                asset_eq_balance   = self.assets_eq_balances_i[asset]
            )

            self.assets_balances_i[asset]   += asset_deposit_balance

            self.assets_eq_balances_i[asset] += pool_tokens_for_asset
    
            deposited_amounts[asset] = asset_deposit_balance.copy()

        # 'Mint' pool tokens for the depositor
        if user not in self.pool_tokens_distribution_i:
            self.pool_tokens_distribution_i[user] = self.uint_type(0)

        self.pool_tokens_distribution_i[user] += pool_tokens_amount
        self.pool_tokens_supply_i             += pool_tokens_amount

        return deposited_amounts



    def _deposit_f(
        self,
        pool_tokens_amount : float,
        user               : UserId,
        timestamp          : int
    ) -> Dict[AssetId, float]:

        # Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
        # upwards by depositing changes the limit.
        self.update_liquidity_units_inflow_f(0, timestamp)

        deposited_amounts: Dict[AssetId, float] = {}

        for asset in self.assets:
            
            pool_tokens_for_asset = (self.assets_eq_balances_f[asset] * pool_tokens_amount) / self.pool_tokens_supply_f
        
            asset_deposit_balance = calc_asset_amount_for_pool_tokens_f(
                pool_token_balance = pool_tokens_for_asset,
                asset_balance      = self.assets_balances_f[asset],     # Escrowed tokens are NOT subtracted from the total balance => deposits should return less
                asset_eq_balance   = self.assets_eq_balances_f[asset]
            )

            self.assets_balances_f[asset]   += asset_deposit_balance

            self.assets_eq_balances_f[asset] += pool_tokens_for_asset
    
            deposited_amounts[asset] = asset_deposit_balance

        # 'Mint' pool tokens for the depositor
        if user not in self.pool_tokens_distribution_f:
            self.pool_tokens_distribution_f[user] = 0

        self.pool_tokens_distribution_f[user] += pool_tokens_amount
        self.pool_tokens_supply_f             += pool_tokens_amount

        return deposited_amounts



    # Withdraw ******************************************************************************************************************

    def withdraw(
        self,
        pool_tokens_amount : TUint | int,
        user               : UserId,
        timestamp          : int | None = None
    ) -> Dict[AssetId, TUint]:      # TODO create class to hold both integer and float results + accuracy stats?

        timestamp = timestamp or get_current_timestamp()

        pool_tokens_amount = self.uint_type(pool_tokens_amount)

        withdrawn_amounts_i = self._withdraw_i(pool_tokens_amount, user, timestamp)
        self._withdraw_f(pool_tokens_amount.value, user, timestamp)

        return withdrawn_amounts_i


    def _withdraw_i(
        self,
        pool_tokens_amount : TUint,
        user               : UserId,
        timestamp          : TUint | int
    ) -> Dict[AssetId, TUint]:

        # Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
        # downwards by withdrawing changes the limit.
        self.update_liquidity_units_inflow_i(self.uint_type(0), self.uint_type(timestamp))

        initial_pool_tokens_supply_i = self.pool_tokens_supply_i

        self.pool_tokens_distribution_i[user] -= pool_tokens_amount
        self.pool_tokens_supply_i             -= pool_tokens_amount

        
        withdrawn_amounts: Dict[AssetId, TUint] = {}

        for asset in self.assets:
    
            pool_tokens_for_asset = (self.assets_eq_balances_i[asset] * pool_tokens_amount) / initial_pool_tokens_supply_i

            asset_withdrawal_balance = calc_asset_amount_for_pool_tokens_i(
                pool_token_balance = pool_tokens_for_asset,
                asset_balance      = self.assets_balances_i[asset] - self.escrowed_assets_i[asset], # Escrowed tokens ARE subtracted from the total balance => withdrawals should return less
                asset_eq_balance   = self.assets_eq_balances_i[asset]
            )

            self.assets_eq_balances_i[asset] -= pool_tokens_for_asset

            self.assets_balances_i[asset] -= asset_withdrawal_balance

            withdrawn_amounts[asset] = asset_withdrawal_balance.copy()
        
        return withdrawn_amounts


    def _withdraw_f(
        self,
        pool_tokens_amount : int,
        user               : UserId,
        timestamp          : int
    ) -> Dict[AssetId, float]:

        # Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
        # downwards by withdrawing changes the limit.
        self.update_liquidity_units_inflow_f(0, timestamp)

        initial_pool_tokens_supply_f = self.pool_tokens_supply_f

        self.pool_tokens_distribution_f[user] -= pool_tokens_amount
        self.pool_tokens_supply_f             -= pool_tokens_amount

        
        withdrawn_amounts: Dict[AssetId, float] = {}

        for asset in self.assets:
    
            pool_tokens_for_asset = (self.assets_eq_balances_f[asset] * pool_tokens_amount) / initial_pool_tokens_supply_f

            asset_withdrawal_balance = calc_asset_amount_for_pool_tokens_f(
                pool_token_balance = pool_tokens_for_asset,
                asset_balance      = self.assets_balances_f[asset] - self.escrowed_assets_f[asset], # Escrowed tokens ARE subtracted from the total balance => withdrawals should return less
                asset_eq_balance   = self.assets_eq_balances_f[asset]
            )

            self.assets_eq_balances_f[asset] -= pool_tokens_for_asset

            self.assets_balances_f[asset] -= asset_withdrawal_balance

            withdrawn_amounts[asset] = asset_withdrawal_balance
        
        return withdrawn_amounts


    # Local swap ****************************************************************************************************************

    def local_swap(
        self,
        from_asset    : AssetId,
        to_asset      : AssetId,
        amount        : TUint | int,
        minimum_yield : TUint | int
    ) -> TUint | None:

        assert from_asset in self.assets
        assert to_asset in self.assets

        amount        = self.uint_type(amount)
        minimum_yield = self.uint_type(minimum_yield)

        result_i = self._local_swap_i(from_asset, to_asset, amount, minimum_yield)

        if not result_i:    # TODO is this the desired behaviour?
            return None
        
        result_f = self._local_swap_f(from_asset, to_asset, amount.value, minimum_yield.value)

        if not result_f:    # TODO is this the desired behaviour?
            pass
    
        return result_i


    def _local_swap_i(
        self,
        from_asset    : AssetId,
        to_asset      : AssetId,
        amount        : TUint,
        minimum_yield : TUint
    ) -> TUint | None:

        assert from_asset in self.assets
        assert to_asset in self.assets

        output_amount = self.uint_type(full_swap_i(
            Uint256(amount),
            Uint256(self.assets_balances_i[from_asset]),
            Uint256(self.assets_weights_i[from_asset]),
            Uint256(self.assets_balances_i[to_asset] - self.escrowed_assets_i[to_asset]),
            Uint256(self.assets_weights_i[to_asset]),
            self.amplification_i_x64
        ))

        if output_amount < minimum_yield: return None
        
        self.assets_balances_i[from_asset] += amount
        self.assets_balances_i[to_asset]   -= output_amount

        # For amplified pools only
        if self.units_inflow_amplification_i_x64 is not None:
            self.max_units_inflow_i_x64 += mul_x64(
                self.units_inflow_amplification_i_x64,
                self.get_units_inflow_capacity_i_x64(
                    self.assets_balances_i[from_asset] - amount,        # BEFORE
                    self.assets_balances_i[from_asset],
                    from_asset
                )
            ) - mul_x64(
                self.units_inflow_amplification_i_x64,
                self.get_units_inflow_capacity_i_x64(
                    self.assets_balances_i[to_asset] + output_amount,   # BEFORE
                    self.assets_balances_i[to_asset],
                    to_asset
                )
            )

        return output_amount


    def _local_swap_f(
        self,
        from_asset    : AssetId,
        to_asset      : AssetId,
        amount        : int,
        minimum_yield : int
    ) -> float | None:

        assert from_asset in self.assets
        assert to_asset in self.assets

        output_amount = full_swap_f(
            amount,
            self.assets_balances_f[from_asset],
            self.assets_weights_f[from_asset],
            self.assets_balances_f[to_asset] - self.escrowed_assets_f[to_asset],
            self.assets_weights_f[to_asset],
            self.amplification_f
        )

        if output_amount < minimum_yield: return None
        
        self.assets_balances_f[from_asset] += amount
        self.assets_balances_f[to_asset]   -= output_amount

        # For amplified pools only
        if self.units_inflow_amplification_f is not None:
            self.max_units_inflow_f += (
                self.units_inflow_amplification_f *
                self.get_units_inflow_capacity_f(
                    self.assets_balances_f[from_asset] - amount,        # BEFORE
                    self.assets_balances_f[from_asset],
                    from_asset
                )
            ) - (
                self.units_inflow_amplification_f *
                self.get_units_inflow_capacity_f(
                    self.assets_balances_f[to_asset] + output_amount,   # BEFORE
                    self.assets_balances_f[to_asset],
                    to_asset
                )
            )

        return output_amount



    # Out swap ******************************************************************************************************************

    def out_swap(
        self,
        from_asset     : AssetId,
        amount         : TUint | int,
        source_swap_id : int
    ) -> Uint256:

        assert from_asset in self.assets

        amount = self.uint_type(amount)
        
        units_x64 = self._out_swap_i(from_asset, amount, source_swap_id)

        self._out_swap_f(from_asset, amount.value, source_swap_id)    # TODO what to do with this output? what if this fails?

        return units_x64
    

    def _out_swap_i(
        self,
        from_asset     : AssetId,
        amount         : TUint,
        source_swap_id : int
    ) -> Uint256:

        units_x64 = out_swap_i_x64(
            Uint256(amount),
            Uint256(self.assets_balances_i[from_asset]),
            Uint256(self.assets_weights_i[from_asset]),
            self.amplification_i_x64
        )

        # Escrow received assets
        assert source_swap_id not in self.escrows_i, "source_swap_id already in use."
        self.escrowed_assets_i[from_asset] += amount
        self.escrows_i[source_swap_id] = (from_asset, amount)

        self.assets_balances_i[from_asset] += amount

        # For amplified pools only
        if self.units_inflow_amplification_i_x64 is not None:
            self.unit_tracker_i_x64 += self.int_type(units_x64)

            self.max_units_inflow_i_x64 += mul_x64(
                self.units_inflow_amplification_i_x64,
                self.get_units_inflow_capacity_i_x64(
                    self.assets_balances_i[from_asset] - amount,
                    self.assets_balances_i[from_asset],
                    from_asset
                )
            )
        
        # Incoming swaps are subtracted from the net pool unit flow. It is assumed that if the router is fraudulent, 
        # no one will execute a trade. Hence, if people swap into the pool, it is expected that there is exactly that 
        # 'inswapped' amount of trust in the pool. Otherwise there would be effectively a maximum allowed daily cross 
        # chain volume, which is bad for liquidity providers.
        if self.current_units_inflow_i_x64 > units_x64:
            self.current_units_inflow_i_x64 = self.current_units_inflow_i_x64 - units_x64
        else:
            self.current_units_inflow_i_x64 = Uint256(0)

        return units_x64
    

    def _out_swap_f(
        self,
        from_asset     : AssetId,
        amount         : int,
        source_swap_id : int
    ) -> float:

        units = out_swap_f(
            amount,
            self.assets_balances_f[from_asset],
            self.assets_weights_f[from_asset],
            self.amplification_f
        )

        # Escrow received assets
        assert source_swap_id not in self.escrows_f, "source_swap_id already in use."
        self.escrowed_assets_f[from_asset] += amount
        self.escrows_f[source_swap_id] = (from_asset, amount)

        self.assets_balances_f[from_asset] += amount

        # For amplified pools only
        if self.units_inflow_amplification_f is not None:
            self.unit_tracker_f += units

            self.max_units_inflow_f += self.units_inflow_amplification_f * self.get_units_inflow_capacity_f(
                self.assets_balances_f[from_asset] - amount,
                self.assets_balances_f[from_asset],
                from_asset
            )
        
        # Incoming swaps are subtracted from the net pool unit flow. It is assumed that if the router is fraudulent, 
        # no one will execute a trade. Hence, if people swap into the pool, it is expected that there is exactly that 
        # 'inswapped' amount of trust in the pool. Otherwise there would be effectively a maximum allowed daily cross 
        # chain volume, which is bad for liquidity providers.
        if self.current_units_inflow_f > units:
            self.current_units_inflow_f = self.current_units_inflow_f - units
        else:
            self.current_units_inflow_f = 0

        return units
    


    def out_swap_ack(
        self,
        source_swap_id: int
    ) -> None:

        self._out_swap_ack_i(source_swap_id)

        self._out_swap_ack_f(source_swap_id)
    

    def _out_swap_ack_i(
        self,
        source_swap_id: int
    ) -> None:

        assert source_swap_id in self.escrows_i, "swap ack: source_swap_id does not exist"

        (escrowed_asset_id, escrowed_amount) = self.escrows_i[source_swap_id]

        self.escrowed_assets_i[escrowed_asset_id] -= escrowed_amount

        del self.escrows_i[source_swap_id]
    

    def _out_swap_ack_f(
        self,
        source_swap_id: int
    ) -> None:

        assert source_swap_id in self.escrows_f, "swap ack: source_swap_id does not exist"

        (escrowed_asset_id, escrowed_amount) = self.escrows_f[source_swap_id]

        self.escrowed_assets_f[escrowed_asset_id] -= escrowed_amount

        del self.escrows_f[source_swap_id]
    


    def out_swap_timeout(
        self,
        source_swap_id: int
    ) -> None:

        self._out_swap_timeout_i(source_swap_id)

        self._out_swap_timeout_f(source_swap_id)
    

    def _out_swap_timeout_i(
        self,
        source_swap_id: int
    ) -> None:

        assert source_swap_id in self.escrows_i, "swap timeout: source_swap_id does not exist"

        (escrowed_asset_id, escrowed_amount) = self.escrows_i[source_swap_id]

        self.escrowed_assets_i[escrowed_asset_id] -= escrowed_amount
        self.assets_balances_i[escrowed_asset_id] -= escrowed_amount

        del self.escrows_i[source_swap_id]
    

    def _out_swap_timeout_f(
        self,
        source_swap_id: int
    ) -> None:

        assert source_swap_id in self.escrows_f, "swap timeout: source_swap_id does not exist"

        (escrowed_asset_id, escrowed_amount) = self.escrows_f[source_swap_id]

        self.escrowed_assets_f[escrowed_asset_id] -= escrowed_amount
        self.assets_balances_f[escrowed_asset_id] -= escrowed_amount

        del self.escrows_f[source_swap_id]



    # In swap *******************************************************************************************************************

    def in_swap(
        self,
        to_asset  : AssetId,
        units_x64 : Uint256,
        timestamp : int | None = None
    ) -> TUint:

        assert to_asset in self.assets

        timestamp = timestamp or get_current_timestamp()

        asset_amount = self._in_swap_i(to_asset, units_x64, self.uint_type(timestamp))

        self._in_swap_f(to_asset, units_x64.value / 2**64, timestamp)    # TODO what if this fails?

        assert self.current_units_inflow_timestamp_i == self.current_units_inflow_timestamp_f

        return asset_amount


    def _in_swap_i(
        self,
        to_asset  : AssetId,
        units_x64 : Uint256,
        timestamp : TUint
    ) -> TUint:

        self.update_units_inflow_i(units_x64, timestamp)

        output_amount = self.uint_type(in_swap_i(
            units_x64,
            Uint256(self.assets_balances_i[to_asset] - self.escrowed_assets_i[to_asset]),
            Uint256(self.assets_weights_i[to_asset]),
            self.amplification_i_x64
        ))

        # For amplified pools only
        if self.units_inflow_amplification_i_x64 is not None:
            self.update_units_inflow_i(
                units_x64,
                timestamp
            )

            self.unit_tracker_i_x64 -= self.int_type(units_x64)

            self.max_units_inflow_i_x64 -= mul_x64(
                self.units_inflow_amplification_i_x64,
                self.get_units_inflow_capacity_i_x64(
                    self.assets_balances_i[to_asset],
                    self.assets_balances_i[to_asset] - output_amount,
                    to_asset
                )
            )

        self.assets_balances_i[to_asset] -= output_amount

        return output_amount


    def _in_swap_f(
        self,
        to_asset : AssetId,
        units    : float,
        timestamp : int
    ) -> float:

        self.update_units_inflow_f(units, timestamp)

        output_amount = in_swap_f(
            units,
            self.assets_balances_f[to_asset] - self.escrowed_assets_f[to_asset],
            self.assets_weights_f[to_asset],
            self.amplification_f
        )

        # For amplified pools only
        if self.units_inflow_amplification_f is not None:
            self.update_units_inflow_f(
                units,
                timestamp
            )

            self.unit_tracker_f -= units

            self.max_units_inflow_f -= self.units_inflow_amplification_f * self.get_units_inflow_capacity_f(
                self.assets_balances_f[to_asset],
                self.assets_balances_f[to_asset] - output_amount,
                to_asset
            )

        self.assets_balances_f[to_asset] -= output_amount

        return output_amount



    # Out liquidity swap ********************************************************************************************************

    def out_liquidity_swap(
        self,
        amount : TUint | int,
        user   : UserId
    ) -> Uint256:

        amount = self.uint_type(amount)
        
        units_x64 = self._out_liquidity_swap_i(amount, user)

        self._out_liquidity_swap_f(amount.value, user)    # TODO what to do with this output? what if this fails?

        return units_x64
    

    def _out_liquidity_swap_i(
        self,
        amount : TUint,
        user   : UserId
    ) -> Uint256:

        liquidity_units_x64 = Uint256(0)

        for asset in self.assets:

            asset_eq_balance = self.assets_eq_balances_i[asset]
            pool_tokens_for_asset = (amount * asset_eq_balance) / self.pool_tokens_supply_i

            liquidity_units_x64 += calc_out_liquidity_swap_i_x64(
                Uint256(pool_tokens_for_asset),
                Uint256(asset_eq_balance),
                Uint256(self.assets_weights_i[asset]),
                self.amplification_i_x64
            )

            self.assets_eq_balances_i[asset] -= pool_tokens_for_asset
        
        # 'Burn' pool tokens
        self.pool_tokens_distribution_i[user] -= amount
        self.pool_tokens_supply_i             -= amount

        # Correct the routing security limit. (To increase the maximum allowed daily volume)
        if self.current_liquidity_inflow_i > amount:
            self.current_liquidity_inflow_i -= amount
        else:
            self.current_liquidity_inflow_i = self.uint_type(0)

        return liquidity_units_x64
    

    def _out_liquidity_swap_f(
        self,
        amount : int,
        user   : UserId
    ) -> float:

        liquidity_units = 0

        for asset in self.assets:

            asset_eq_balance = self.assets_eq_balances_f[asset]
            pool_tokens_for_asset = (amount * asset_eq_balance) / self.pool_tokens_supply_f

            liquidity_units += calc_out_liquidity_swap_f(
                amount,
                asset_eq_balance,
                self.assets_weights_f[asset],
                self.amplification_f
            )

            self.assets_eq_balances_f[asset] -= pool_tokens_for_asset
        
        # 'Burn' pool tokens
        self.pool_tokens_distribution_f[user] -= amount
        self.pool_tokens_supply_f             -= amount

        # Correct the routing security limit. (To increase the maximum allowed daily volume)
        if self.current_liquidity_inflow_f > amount:
            self.current_liquidity_inflow_f -= amount
        else:
            self.current_liquidity_inflow_f = 0

        return liquidity_units


    # In liquidity swap *********************************************************************************************************

    def in_liquidity_swap(
        self,
        liquidity_units_x64 : Uint256,
        user                : UserId,
        timestamp           : int | None = None
    ) -> TUint:

        timestamp = timestamp or get_current_timestamp()

        if user not in self.pool_tokens_distribution_i:
            assert user not in self.pool_tokens_distribution_f
            self.pool_tokens_distribution_i[user] = self.uint_type(0)
            self.pool_tokens_distribution_f[user] = 0

        asset_amount = self._in_liquidity_swap_i(liquidity_units_x64, user, self.uint_type(timestamp))

        self._in_liquidity_swap_f(liquidity_units_x64.value / 2**64, user, timestamp)    # TODO what if this fails?

        assert self.current_liquidity_inflow_timestamp_i == self.current_liquidity_inflow_timestamp_f

        return asset_amount


    def _in_liquidity_swap_i(
        self,
        liquidity_units_x64 : Uint256,
        user                : UserId,
        timestamp           : TUint
    ) -> TUint:

        aggregate_weight_x64 = Uint256(0)

        one_minus_amp_x64 = ONE_X64 - (self.amplification_i_x64 or Uint256(2**64))

        for asset in self.assets:

            asset_eq_balance = self.assets_eq_balances_i[asset]

            if self.amplification_i_x64 is None:
                aggregate_weight_x64 += Uint256(self.assets_weights_i[asset]) << 64

            else:
                aggregate_weight_x64 += Uint256(self.assets_weights_i[asset]) * pow_x64(Uint256(asset_eq_balance) << 64, one_minus_amp_x64)

        # Compute the 'received' pool tokens corresponding to the first asset of the pool
        ref_asset = self.assets[0]
        ref_asset_pool_tokens = calc_in_liquidity_swap_i(
            liquidity_units_x64,
            Uint256(self.assets_eq_balances_i[ref_asset]),
            aggregate_weight_x64,
            self.amplification_i_x64
        )
        
        # Compute the total pool tokens 'received' from the ones corresponding to the first asset
        pool_tokens_supply = Uint256(self.pool_tokens_supply_i)
        
        total_pool_tokens = (ref_asset_pool_tokens * pool_tokens_supply) / Uint256(self.assets_eq_balances_i[ref_asset])

        self.assets_eq_balances_i[ref_asset] += ref_asset_pool_tokens.value

        
        for asset in self.assets[1:]:

            asset_eq_balance = self.assets_eq_balances_i[asset]
            
            self.assets_eq_balances_i[asset] += (
                (total_pool_tokens * Uint256(asset_eq_balance)) / pool_tokens_supply
            ).value

        # Verify and update the security limit
        self.update_liquidity_units_inflow_i(
            self.uint_type(total_pool_tokens),
            timestamp
        )

        # 'Mint' pool tokens
        self.pool_tokens_distribution_i[user] += total_pool_tokens.value
        self.pool_tokens_supply_i             += total_pool_tokens.value

        return self.uint_type(total_pool_tokens.value)


    def _in_liquidity_swap_f(
        self,
        liquidity_units : float,
        user            : UserId,
        timestamp       : int
    ) -> float:

        aggregate_weight = 0

        one_minus_amp = 1 - (self.amplification_f or 1)

        for asset in self.assets:

            asset_eq_balance = self.assets_eq_balances_f[asset]

            if self.amplification_f is None:
                aggregate_weight += self.assets_weights_f[asset]

            else:
                aggregate_weight += self.assets_weights_f[asset] * asset_eq_balance**one_minus_amp

        # Compute the 'received' pool tokens corresponding to the first asset of the pool
        ref_asset = self.assets[0]
        asset_0_pool_tokens = calc_in_liquidity_swap_f(
            liquidity_units,
            self.assets_eq_balances_f[ref_asset],
            aggregate_weight,
            self.amplification_f
        )
        
        # Compute the total pool tokens 'received' from the ones corresponding to the first asset
        pool_tokens_supply = self.pool_tokens_supply_f
        
        total_pool_tokens = (asset_0_pool_tokens * pool_tokens_supply) / self.assets_eq_balances_f[ref_asset]

        self.assets_eq_balances_f[ref_asset] += asset_0_pool_tokens

        
        for asset in self.assets[1:]:

            asset_eq_balance = self.assets_eq_balances_f[asset]
            
            self.assets_eq_balances_f[ref_asset] += (
                (total_pool_tokens * asset_eq_balance) / pool_tokens_supply
            )

        # Verify and update the security limit
        self.update_liquidity_units_inflow_f(
            total_pool_tokens,
            timestamp
        )

        # 'Mint' pool tokens
        self.pool_tokens_distribution_f[user] += total_pool_tokens
        self.pool_tokens_supply_f             += total_pool_tokens

        return total_pool_tokens
    

    # Fees **********************************************************************************************************************

    def distribute_fees(self) -> None:
        self._distribute_fees_i()
        self._distribute_fees_f()

    def _distribute_fees_i(self) -> None:

        assert self.amplification_i_x64 is not None
        
        one_minus_amp_x64 = ONE_X64 - self.amplification_i_x64

        aggregate_weight_x64 = Uint256(0)
        calc_outstanding_units_x64 = self.int_type(0)

        for asset in self.assets:

            asset_eq_balance = self.assets_eq_balances_i[asset]
            asset_balance    = self.assets_balances_i[asset]

            if asset_balance > asset_eq_balance:
                calc_outstanding_units_x64 += self.int_type(calc_out_liquidity_swap_i_x64(  # type: ignore
                    Uint256(asset_balance - asset_eq_balance),
                    Uint256(asset_eq_balance),
                    Uint256(self.assets_weights_i[asset]),
                    one_minus_amp_x64
                ))
            else:
                calc_outstanding_units_x64 -= self.int_type(calc_out_liquidity_swap_i_x64(  # type: ignore
                    Uint256(asset_eq_balance - asset_balance),
                    Uint256(asset_balance),
                    Uint256(self.assets_weights_i[asset]),
                    one_minus_amp_x64
                ))
            
            aggregate_weight_x64 += Uint256(self.assets_weights_i[asset]) * pow_x64(
                Uint256(asset_eq_balance << 64),
                one_minus_amp_x64
            )
        
        calc_outstanding_units_x64 = self.unit_tracker_i_x64 - calc_outstanding_units_x64

        # Compute the 'received' pool tokens corresponding to the first asset of the pool
        ref_asset = self.assets[0]
        ref_asset_pool_tokens = calc_in_liquidity_swap_i(
            Uint256(calc_outstanding_units_x64),
            Uint256(self.assets_eq_balances_i[ref_asset]),
            aggregate_weight_x64,
            self.amplification_i_x64
        )
        
        # Compute the total pool tokens from the ones corresponding to the first asset
        pool_tokens_supply = Uint256(self.pool_tokens_supply_i)
        
        total_pool_tokens = (ref_asset_pool_tokens * pool_tokens_supply) / Uint256(self.assets_eq_balances_i[ref_asset])

        # Update the eq balances
        self.assets_eq_balances_i[ref_asset] += ref_asset_pool_tokens.value

        
        for asset in self.assets[1:]:

            asset_eq_balance = self.assets_eq_balances_i[asset]
            
            self.assets_eq_balances_i[asset] += self.uint_type(total_pool_tokens) * asset_eq_balance

    def _distribute_fees_f(self) -> None:

        assert self.amplification_f is not None
        
        one_minus_amp = 1 - self.amplification_f

        aggregate_weight = 0
        calc_outstanding_units = 0

        for asset in self.assets:

            asset_eq_balance = self.assets_eq_balances_f[asset]
            asset_balance    = self.assets_balances_f[asset]

            if asset_balance > asset_eq_balance:
                calc_outstanding_units += calc_out_liquidity_swap_f(
                    asset_balance - asset_eq_balance,
                    asset_eq_balance,
                    self.assets_weights_f[asset],
                    one_minus_amp
                )
            else:
                calc_outstanding_units -= calc_out_liquidity_swap_f(
                    asset_eq_balance - asset_balance,
                    asset_balance,
                    self.assets_weights_f[asset],
                    one_minus_amp
                )
            
            aggregate_weight += self.assets_weights_f[asset] * asset_eq_balance**one_minus_amp
        
        calc_outstanding_units = self.unit_tracker_f - calc_outstanding_units

        # Compute the 'received' pool tokens corresponding to the first asset of the pool
        ref_asset = self.assets[0]
        ref_asset_pool_tokens = calc_in_liquidity_swap_f(
            calc_outstanding_units,
            self.assets_eq_balances_f[ref_asset],
            aggregate_weight,
            self.amplification_f
        )
        
        # Compute the total pool tokens from the ones corresponding to the first asset
        pool_tokens_supply = self.pool_tokens_supply_f
        
        total_pool_tokens = (ref_asset_pool_tokens * pool_tokens_supply) / self.assets_eq_balances_f[ref_asset]

        # Update the eq balances
        self.assets_eq_balances_f[ref_asset] += ref_asset_pool_tokens

        
        for asset in self.assets[1:]:

            asset_eq_balance = self.assets_eq_balances_f[asset]
            
            self.assets_eq_balances_f[asset] += total_pool_tokens * asset_eq_balance



    # Security helpers **********************************************************************************************************
    
    # Update units inflow
    def update_units_inflow_i(
        self,
        units_inflow_i_x64: Uint256,
        timestamp: TUint
    ) -> None:
        
        # If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if timestamp > self.current_units_inflow_timestamp_i + self.DECAY_RATE:
            if units_inflow_i_x64 > self.max_units_inflow_i_x64:
                raise Exception("Swap limit exceeded")

            self.current_units_inflow_i_x64       = units_inflow_i_x64.copy()
            self.current_units_inflow_timestamp_i = timestamp

            return

        # Compute how much inflow has decayed since last update

        decayed_inflow_x64 = (
            self.max_units_inflow_i_x64 * Uint256(timestamp - self.current_units_inflow_timestamp_i)
        ) / self.DECAY_RATE

        # If the current inflow is less then the (max allowed) decayed one
        if self.current_units_inflow_i_x64 <= decayed_inflow_x64 :
            if units_inflow_i_x64 > self.max_units_inflow_i_x64:
                raise Exception("Swap limit exceeded")

            self.current_units_inflow_i_x64 = units_inflow_i_x64.copy()

        # If some of the current inflow still matters
        else:
            new_net_units_inflow_x64 = (self.current_units_inflow_i_x64 - decayed_inflow_x64) + units_inflow_i_x64

            if new_net_units_inflow_x64 > self.max_units_inflow_i_x64:
                raise Exception("Swap limit exceeded")

            self.current_units_inflow_i_x64 = new_net_units_inflow_x64

        self.current_units_inflow_timestamp_i = timestamp


    def update_units_inflow_f(
        self,
        units_inflow_f: float,
        timestamp: int
    ) -> None:
        
        # If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if timestamp > self.current_units_inflow_timestamp_f + self.DECAY_RATE:
            if units_inflow_f > self.max_units_inflow_f:
                raise Exception("Swap limit exceeded")

            self.current_units_inflow_f           = units_inflow_f
            self.current_units_inflow_timestamp_f = timestamp

            return

        # Compute how much inflow has decayed since last update

        decayed_inflow = (
            self.max_units_inflow_f * (timestamp - self.current_units_inflow_timestamp_f)
        ) / self.DECAY_RATE

        # If the current inflow is less then the (max allowed) decayed one
        if self.current_units_inflow_f <= decayed_inflow :
            if units_inflow_f > self.max_units_inflow_f:
                raise Exception("Swap limit exceeded")

            self.current_units_inflow_f = units_inflow_f

        # If some of the current inflow still matters
        else:
            new_net_units_inflow = (self.current_units_inflow_f - decayed_inflow) + units_inflow_f

            if new_net_units_inflow > self.max_units_inflow_f:
                raise Exception("Swap limit exceeded")

            self.current_units_inflow_f = new_net_units_inflow

        self.current_units_inflow_timestamp_f = timestamp
    

    def update_liquidity_units_inflow_i(
        self,
        pool_tokens_flow: TUint,
        timestamp: TUint
    ) -> None:

        # Allows 1/3 of the pool to be drained through liquidity swaps
        max_pool_tokens_flow = self.pool_tokens_supply_i / 2

        # If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if timestamp > self.current_liquidity_inflow_timestamp_i + self.DECAY_RATE:
            if pool_tokens_flow > max_pool_tokens_flow:
                raise Exception("Liquidity swap limit exceeded")

            self.current_liquidity_inflow_i           = pool_tokens_flow
            self.current_liquidity_inflow_timestamp_i = timestamp

        # Compute how much inflow has decayed since last update
        decayed_inflow = max_pool_tokens_flow * (
            timestamp - self.current_liquidity_inflow_timestamp_i
        ) / self.DECAY_RATE

        # If the current inflow is less then the (max allowed) decayed one
        if self.current_liquidity_inflow_i <= decayed_inflow:
            if pool_tokens_flow > max_pool_tokens_flow:
                raise Exception("Liquidity swap limit exceeded")

            self.current_liquidity_inflow_i = pool_tokens_flow

        # If some of the current inflow still matters
        else:
            new_net_liquidity_inflow = (self.current_liquidity_inflow_i - decayed_inflow) + pool_tokens_flow;  # Substraction is safe, as current_liquidity_inflow > decayed_inflow is guaranteed by if statement

            if new_net_liquidity_inflow > max_pool_tokens_flow:
                raise Exception("Liquidity swap limit exceeded")

            self.current_liquidity_inflow_i = new_net_liquidity_inflow

        self.current_liquidity_inflow_timestamp_i = timestamp
    

    def update_liquidity_units_inflow_f(
        self,
        pool_tokens_flow: float,
        timestamp: int
    ) -> None:

        # Allows 1/3 of the pool to be drained through liquidity swaps
        max_pool_tokens_flow = self.pool_tokens_supply_f / 2

        # If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if timestamp > self.current_liquidity_inflow_timestamp_f + self.DECAY_RATE:
            if pool_tokens_flow > max_pool_tokens_flow:
                raise Exception("Liquidity swap limit exceeded")

            self.current_liquidity_inflow_f           = pool_tokens_flow
            self.current_liquidity_inflow_timestamp_f = timestamp

        # Compute how much inflow has decayed since last update
        decayed_inflow = max_pool_tokens_flow * (
            timestamp - self.current_liquidity_inflow_timestamp_f
        ) / self.DECAY_RATE

        # If the current inflow is less then the (max allowed) decayed one
        if self.current_liquidity_inflow_f <= decayed_inflow:
            if pool_tokens_flow > max_pool_tokens_flow:
                raise Exception("Liquidity swap limit exceeded")

            self.current_liquidity_inflow_f = pool_tokens_flow

        # If some of the current inflow still matters
        else:
            new_net_liquidity_inflow = (self.current_liquidity_inflow_f - decayed_inflow) + pool_tokens_flow;  # Substraction is safe, as current_liquidity_inflow > decayed_inflow is guaranteed by if statement

            if new_net_liquidity_inflow > max_pool_tokens_flow:
                raise Exception("Liquidity swap limit exceeded")

            self.current_liquidity_inflow_f = new_net_liquidity_inflow

        self.current_liquidity_inflow_timestamp_f = timestamp


    def get_units_inflow_capacity_i_x64(
        self,
        old_balance: TUint,
        new_balance: TUint,
        asset: AssetId
    ) -> Uint256 :
        if old_balance == new_balance:
            return Uint256(0)

        assert self.amplification_i_x64 is not None
        one_minus_amp_x64 = ONE_X64 - self.amplification_i_x64

        if old_balance < new_balance:
            return Uint256(self.assets_weights_i[asset]) * (
                pow_x64(Uint256(new_balance << 64), one_minus_amp_x64) - pow_x64(Uint256(old_balance << 64), one_minus_amp_x64)
            )

        return Uint256(self.assets_weights_i[asset]) * (
            pow_x64(Uint256(old_balance << 64), one_minus_amp_x64) - pow_x64(Uint256(new_balance << 64), one_minus_amp_x64)
        )

    def get_units_inflow_capacity_f(
        self,
        old_balance: float,
        new_balance: float,
        asset: AssetId
    ) -> float :
        if old_balance == new_balance:
            return 0

        assert self.amplification_f is not None
        one_minus_amp_f = 1 - self.amplification_f

        if old_balance < new_balance:
            return self.assets_weights_f[asset] * (
                new_balance**one_minus_amp_f - old_balance**one_minus_amp_f
            )

        return self.assets_weights_f[asset] * (
            old_balance**one_minus_amp_f - new_balance**one_minus_amp_f
        )


# Helpers
def get_current_timestamp() -> int:
    return int(time.time())