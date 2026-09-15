pub struct InstructionStream{
     pub stream:mpsc::Receiver<Result<Instruction, InstructionStreamError>>
}