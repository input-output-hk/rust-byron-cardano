use super::*;

/// Take the given input collections and select the inputs in the given order
///
/// This is the least interesting algorithm, it is however very simple and
/// provide the interesting property to be reproducible.
///
#[derive(Debug, Clone)]
pub struct HeadFirst<Addressing>(Vec<Input<Addressing>>);
impl<Addressing> From<Vec<Input<Addressing>>> for HeadFirst<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self { HeadFirst(inputs) }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for HeadFirst<Addressing> {
    fn select_input<F>( &mut self
                      , _fee_algorithm: &F
                      , _estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        if self.0.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.0.remove(0)))
        }
    }
}

/// Takes the large inputs first.
///
/// About the same as `FirstMatchFirst` but sort the input list
/// to take the largest inputs first.
///
#[derive(Debug, Clone)]
pub struct LargestFirst<Addressing>(HeadFirst<Addressing>);
impl<Addressing> From<Vec<Input<Addressing>>> for LargestFirst<Addressing> {
    fn from(mut inputs: Vec<Input<Addressing>>) -> Self {
        inputs.sort_unstable_by(|i1, i2| i2.value.value.cmp(&i1.value.value));
        LargestFirst(HeadFirst::from(inputs))
    }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for LargestFirst<Addressing> {
    fn select_input<F>( &mut self
                      , fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        self.0.select_input(fee_algorithm, estimated_needed_output)
    }
}

/// This input selection strategy will accumulates inputs until the target value
/// is matched, except it ignores the inputs that go over the target value
pub struct Blackjack<Addressing>(LargestFirst<Addressing>);
impl<Addressing> Blackjack<Addressing> {
    #[inline]
    fn find_index_where_value_less_than(&self, needed_output: Coin) -> Option<usize> {
        ((self.0).0).0.iter().position(|input| input.value.value <= needed_output)
    }
}
impl<Addressing> From<Vec<Input<Addressing>>> for Blackjack<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self {
        Blackjack(LargestFirst::from(inputs))
    }
}
impl<Addressing> InputSelectionAlgorithm<Addressing> for Blackjack<Addressing> {
    fn select_input<F>( &mut self
                      , _fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        let index = self.find_index_where_value_less_than(estimated_needed_output);
        match index {
            None => Ok(None),
            Some(index) => {
                Ok(Some(((self.0).0).0.remove(index)))
            }
        }
    }
}

/// Blackjack with Backup (Large input first)
///
/// Considering a collection of input (ordered large input to small input), we will take
/// the first inputs that are below the expected amount. This is in order to minimise using
/// large inputs for small transactions.
///
/// Once there is no longer inputs below the targeted output, it will fallback to `LargeInputFirst`.
///
enum BlackjackWithBackupPlanE<Addressing> {
    Blackjack(Blackjack<Addressing>),
    BackupPlan(LargestFirst<Addressing>)
}
pub struct BlackjackWithBackupPlan<Addressing>(BlackjackWithBackupPlanE<Addressing>);
impl<Addressing> From<Vec<Input<Addressing>>> for BlackjackWithBackupPlan<Addressing> {
    fn from(inputs: Vec<Input<Addressing>>) -> Self {
        BlackjackWithBackupPlan(
        BlackjackWithBackupPlanE::Blackjack(
            Blackjack::from(inputs)
        ))
    }
}
impl<Addressing: Clone> InputSelectionAlgorithm<Addressing> for BlackjackWithBackupPlan<Addressing> {
    fn select_input<F>( &mut self
                      , fee_algorithm: &F
                      , estimated_needed_output: Coin
                      )
        -> Result<Option<Input<Addressing>>>
      where F: FeeAlgorithm
    {
        let input_1 = match &mut self.0 {
            BlackjackWithBackupPlanE::Blackjack(ref mut v) => {
                v.select_input(fee_algorithm, estimated_needed_output)?
            }
            BlackjackWithBackupPlanE::BackupPlan(ref mut v) => {
                v.select_input(fee_algorithm, estimated_needed_output)?
            }
        };

        if input_1.is_none() {
            let backup = if let BlackjackWithBackupPlanE::Blackjack(Blackjack(lif)) = &self.0 {
                lif.clone()
            } else {
                return Ok(None)
            };
            self.0 = BlackjackWithBackupPlanE::BackupPlan(backup);
            self.select_input(fee_algorithm, estimated_needed_output)
        } else {
            Ok(input_1)
        }
    }
}
