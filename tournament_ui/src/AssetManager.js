import { generalError } from './state/action';

export default class AssetManager {
    constructor(cb, dispatch) {
        this.assets = {};
        console.log('constructor');
        const assetPromises = [];
        // Load pacmen
        for (let i = 0; i < 1; i++) {
            assetPromises.push(
                this.loadAsset(
                    'pacman' + i,
                    process.env.PUBLIC_URL + 'assets/pacman' + i + '.png'
                )
            );
        }

        // Load ghosts
        for (let i = 0; i < 1; i++) {
            assetPromises.push(
                this.loadAsset(
                    'ghost' + i,
                    process.env.PUBLIC_URL + 'assets/ghost' + i + '.png'
                )
            );
        }

        assetPromises.push(
            this.loadAsset('fruit', process.env.PUBLIC_URL + 'assets/fruit.png')
        );
        assetPromises.push(
            this.loadAsset('power', process.env.PUBLIC_URL + 'assets/power.png')
        );
        Promise.all(assetPromises)
            .then(() => {
                console.log('Assets loaded');
                console.log(this);
                cb(this);
            })
            .catch(err => {
                console.error(err);
                dispatch(
                    generalError(
                        'We couldn\'t load the sprites for the game',
                        'SPRITE_FAILED_LOADING'
                    )
                );
            });
    }

    loadAsset(name, path) {
        return new Promise((res, reject) => {
            const image = new Image();
            image.src = path;
            image.onload = () => {
                this.assets[name] = image;
                res();
            };
            image.onerror = () => {
                reject();
            };
        });
    }

    getAsset(name) {
        return this.assets[name];
    }

    getPacman(variant) {
        if (this.assets['pacman' + variant]) {
            return this.assets['pacman' + variant];
        } else {
            return this.assets['pacman0'];
        }
    }

    getGhost(variant) {
        if (this.assets['ghost' + variant]) {
            return this.assets['ghost' + variant];
        } else {
            return this.assets['ghost0'];
        }
    }
}
