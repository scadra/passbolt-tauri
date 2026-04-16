define_topics!(FolderTopics {
    CREATE => "passbolt.folders.create",
    UPDATE => "passbolt.folders.update",
    DELETE => "passbolt.folders.delete",
    OPEN_DIALOG => "passbolt.folders.move-by-id",
    UPDATE_LOCALSTORAGE => "passbolt.folders.update-local-storage",
    SELECT_AND_SCROLL_TO => "passbolt.folders.select-and-scroll-to",
    MOVE_STRATEGY_REQUEST => "passbolt.folders.move-strategy.request",
    FIND_DETAILS => "passbolt.folders.find-details",
});
