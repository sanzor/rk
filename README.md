important files:
- architecture.drawio
- chat-transcript_1.md  (transcript part 1 using Claude Sonnet)
- chat-transcript_2.md (transcript part 2 using Codex more after decisions and questions)
* there are a number of file samples for tests (also including multiple files in the tests/fixtures)


Some notes:
- check architecture.drawio 
- i have approached the problem in an actor model way , using channels , and the "respond back to sender" (Erlang style - payload contains the ProcessId of who you will respond to ) in the payload

Payload:   { Sender, Instruction }  //sent via channel from the hot path to a consumer (or a consumer group)


Rust Equivalent:

sender.send({respond_back_to,payload}).await
......
let {respond_back_to,payload}=receiver.recv().await?;
let result=process(payload).await?;
respond_back_to.send(result).await;

Erlang equivalent:

receive({PidSender,Payload},State)->
  Result=process(Payload),   //process the instruction
  PidSender ! {result,Result}.  //use the ProcessId/Sender in Rust to send the result back

- once the receiver processes the instruction it sends the instruction outcome back using the "respond back to sender"
- the engine can easily be integrated in a server (injected in the app data) , alongside the client factory
- on the hot path a user can use the said factory to fetch a client and send instruction(s) using it
- the client can be extended to support a batch of instructions , or even a stream (as they are on the same machine)
- the state of the engine is raw (currently in memory only) , typically should be an in memory snapshot of the state of the accounts + regular dumps in a database , + you can also save all the messages if you plan to do Event Sourcing or Audit Trails , or for using caches and hydrating them later on by replaying the database
- there are tests for each component you can run cargo test !!!
- disputes, resolves, and chargebacks apply only to deposits. A withdrawal is
  not a credit that can be held under the balance transitions defined by the
  brief; treating a disputed withdrawal as one would debit available funds a
  second time rather than reverse it. A lifecycle instruction that references
  a withdrawal is therefore ignored as an unknown disputable transaction.

-decisions
- i went for an in memory store for the moment (with arc , mutex ) , thus using a registry to guarantee unicity (if i had the database i would have solved alot of problems - passing this issues to it)
- i went for an sender-receiver based approach in order to decouple the two , as i dont know who sends what , what velocity , but i also had to bound the sender to a pool (like in Erlang an elastic bounded pool of processes).
- i also went (for now for a single consumer worker) which consumes from this message queue that forms (using channels) , will probably need a consumer group depending on the throughput , also will need to bound channels , add telemetry and check performance , delays, queue length 
- for now the state is kept in memory in the "Engine" class decoupled from IO , to be easily testable , IO bound operations (database in the future) will require a new layer of tests with mocks 

