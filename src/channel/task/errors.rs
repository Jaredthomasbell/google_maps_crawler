
#[derive(Debug)]
pub enum Error {
    FailedToBuildCompany,
    FailedToInsertCompany,
    FailedToGetCompany,
    FailedCrawl,
    FailedProspectExtraction,
    FailedToUdateCompany,
    FailedUpdateWorkerState,
}