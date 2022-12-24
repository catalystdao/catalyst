from __future__ import annotations
from functools import cache
from typing import Any, Type, TypeGuard, Tuple, TypeVar

# Used as return type for RealInt methods, so that the return type of the methods matches that of the class that inherits RealInt
#   e.g. Uint256 + Uint256 ==> Uint256 and not RealInt
TRealInt = TypeVar("TRealInt", bound="RealInt") 

class RealInt:

    _value     : int
    _size      : int
    _signed    : bool
    _min_value : int
    _max_value : int


    def __init__(self, value: int | RealInt, size: int, signed=False, mod=False):

        self._size   = size
        self._signed = signed

        self._min_value, self._max_value = compute_int_range(size, signed)

        self.assign_value(value, mod)
    

    @property
    def value(self) -> int:
        return self._value
    
    @property
    def size(self) -> int:
        return self._size

    @property
    def signed(self) -> int:
        return self._signed

    @property
    def min_value(self) -> int:
        return self._min_value
        
    @property
    def max_value(self) -> int:
        return self._max_value


    def assign_value(self, value: int | RealInt, mod = False):

        value = self._value_of(value)

        if mod:
            self._value = value % (self._max_value + 1)
        else:
            if value < self._min_value or value > self._max_value:
                raise OverflowError()
            
            self._value = value
    

    def new(self: TRealInt, value: int | RealInt, mod = False) -> TRealInt:
        if type(self) == RealInt:
            return type(self)(value, self._size, self._signed, mod=mod)
        else:
            return type(self)(value, mod=mod)    # type: ignore

    
    def is_zero(self):
        return self._value == 0

    
    def copy(self: TRealInt) -> TRealInt:
        return self.__copy__()
    

    def cast(self, new_type: Type[TUint | TInt]) -> TUint | TInt:
        return new_type(self.value)


    def overflowing_add(self: TRealInt, other: int | TRealInt ) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value + self._value_of(other), mod=True)


    def overflowing_sub(self: TRealInt, other: int | TRealInt ) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value - self._value_of(other), mod=True)


    def overflowing_mul(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value * self._value_of(other), mod=True)

    
    def _value_of(self, el: int | RealInt) -> int:
        return el._value if is_real_int(el) else el      # type: ignore


    def _is_of_same_type(self, other: RealInt) -> bool:
        return self._size == other._size and self._signed == other._signed

    
    def __copy__(self: TRealInt) -> TRealInt:
        return self.new(self.value)


    def __bool__(self):
        return self._value != 0


    def __add__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value + self._value_of(other))


    def __sub__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value - self._value_of(other))


    def __mul__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value * self._value_of(other))


    def __truediv__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value // self._value_of(other))


    def __floordiv__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value // self._value_of(other))


    def __mod__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value % self._value_of(other))


    def __pow__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value ** self._value_of(other))


    def __lshift__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()

        return self.new(self._value << self._value_of(other))


    def __rshift__(self: TRealInt, other: int | TRealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value >> self._value_of(other))


    def __and__(self: TRealInt, other: int | RealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value & self._value_of(other))


    def __or__(self: TRealInt, other: int | RealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value | self._value_of(other))


    def __xor__(self: TRealInt, other: int | RealInt) -> TRealInt:

        if is_real_int(other) and not self._is_of_same_type(other):
            raise TypeError()
        
        return self.new(self._value ^ self._value_of(other))


    def __lt__(self: TRealInt, other: int | TRealInt) -> bool:

        return self._value < self._value_of(other)


    def __le__(self: TRealInt, other: int | TRealInt) -> bool:

        return self._value <= self._value_of(other)


    def __eq__(self: TRealInt, other: int | TRealInt) -> bool:

        return self._value == self._value_of(other)


    def __ne__(self: TRealInt, other: int | TRealInt) -> bool:

        return self._value != self._value_of(other)


    def __gt__(self: TRealInt, other: int | TRealInt) -> bool:

        return self._value > self._value_of(other)


    def __ge__(self: TRealInt, other: int | TRealInt) -> bool:

        return self._value >= self._value_of(other)



class Uint256(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 256, False, mod=mod)

class Uint128(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 128, False, mod=mod)

class Uint64(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 64, False, mod=mod)

class Uint32(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 32, False, mod=mod)

class Uint16(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 16, False, mod=mod)

class Uint8(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 8, False, mod=mod)



class Int256(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 256, True, mod=mod)

class Int128(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 128, True, mod=mod)

class Int64(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 64, True, mod=mod)

class Int32(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 32, True, mod=mod)

class Int16(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 16, True, mod=mod)

class Int8(RealInt):
    def __init__(self, value: int | RealInt, mod: bool = False):
        super().__init__(value, 8, True, mod=mod)



TUint = TypeVar("TUint", Uint256, Uint128, Uint64, Uint32, Uint16, Uint8)   
TInt  = TypeVar("TInt", Int256, Int128, Int64, Int32, Int16, Int8)   



def is_real_int(el: Any) -> TypeGuard[RealInt]:
    return isinstance(el, RealInt)


@cache
def compute_int_range(size: int, signed: bool) -> Tuple[int, int]:
    return \
        -2**(size-1)     if signed else 0, \
        2**(size-1) - 1  if signed else 2**size - 1