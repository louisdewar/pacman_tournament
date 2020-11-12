export function websocketConnected() {
    return { type: 'WEBSOCKET_CONNECT' };
}

export function websocketClosed() {
    return {
        type: 'GENERAL_ERROR',
        errorType: 'WEBSOCKET_CLOSED',
        error:
            'There\'s an error connecting to the server. We\'ll keep retrying.',
    };
}

export function generalError(error, errorType) {
    return {
        type: 'GENERAL_ERROR',
        errorType,
        error,
    };
}

export function gameOpened(gameID, width, height, baseTiles, entities, food) {
    return {
        gameID,
        width,
        height,
        baseTiles,
        entities,
        food,
        type: 'GAME_OPENED',
    };
}

export function gameClosed(gameID) {
    return { gameID, type: 'GAME_CLOSED' };
}

export function leaderboardUpdate(leaderboard) {
    return { leaderboard, type: 'LEADERBOARD_UPDATE' };
}

export function gameDelta(
    gameID,
    entity_died,
    entity_moved,
    entity_spawned,
    food_eaten,
    food_spawned,
    metadata_changed
) {
    return {
        gameID,
        type: 'GAME_DELTA',
        entity_died,
        entity_moved,
        entity_spawned,
        food_eaten,
        food_spawned,
        metadata_changed,
    };
}
