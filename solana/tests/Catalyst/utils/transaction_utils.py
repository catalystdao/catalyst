from types import TracebackType
from typing import Any, Type, cast
from anchorpy import EventParser, Program, Provider
from solders.signature import Signature
from solders.rpc.responses import GetSignatureStatusesResp
from solana.rpc.commitment import Commitment, Confirmed
from solana.rpc.websocket_api import SolanaWsClientProtocol, connect
from solana.rpc.commitment import Processed

DEFAULT_TX_COMMITMENT  = Confirmed
DEFAULT_SKIP_PREFLIGHT = True

class TransactionError(Exception):
    pass

class UnknownTransactionError(Exception):
    pass


async def confirm_transaction(provider: Provider, tx: Signature, commitment: Commitment = DEFAULT_TX_COMMITMENT) -> GetSignatureStatusesResp:
    confirmation = await provider.connection.confirm_transaction(tx, commitment=commitment)

    try:
        confirmation_error = None if confirmation.value[0] is None else confirmation.value[0].err
    except:
        raise UnknownTransactionError
        
    if confirmation_error is not None:
        raise TransactionError(confirmation_error)


    return confirmation


class TxEventListener():

    ws: SolanaWsClientProtocol

    event_name        : str | None
    event_commitment  : Commitment
    connection_uri    : str
    connection_kwargs : Any

    def __init__(
        self,
        event_name       : str | None = None,
        event_commitment : Commitment = Processed,
        uri              : str = "ws://localhost:8900",
        **kwargs         : Any
    ) -> None:

        self.event_name        = event_name
        self.event_commitment  = event_commitment
        self.connection_uri    = uri
        self.connection_kwargs = kwargs


    async def __aenter__(self):

        self.ws = cast(SolanaWsClientProtocol, await connect(self.connection_uri, **self.connection_kwargs))  # ! 'await connect' important ==> Creates protocol object

        await self.ws.logs_subscribe(commitment=self.event_commitment)
        await self.ws.recv()

        return self
    

    async def get_events(self, program: Program):

        events = []

        data = await self.ws.recv()
        logs: List[str] = data[0].result.value.logs   # type: ignore

        EventParser(program.program_id, program.coder).parse_logs(
            logs,
            lambda event: events.append(event) if self.event_name is None or self.event_name == event.name else None
        )

        return events


    async def __aexit__(
        self,
        exc_type: Type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None
    ):
        await self.ws.close()
