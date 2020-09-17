import socket
import json

class Ai:
    def __init__(self, address, port):
        # self.sock = socket.socket()
#            socket.AF_INET,
#            socket.SOCK_STREAM | socket.SOCK_NONBLOCK)

        print('Connecting to {}:{}'.format(address, port))
        self.sock = socket.create_connection((address, port))

    def start(self):
        buf = ''

        while True:
            index = buf.find('\n')
            if index != -1:
                msg = buf[0:index]
                buf = buf[index + 1:]
                try:
                    data = json.loads(msg)
                    if 'final_score' in data:
                        print('You died your final score was', data['final_score'])
                        return

                    action = self.choose_action(data)
                    print('Playing action', action)
                    self.sock.send(bytes(action, 'utf-8'))
                except json.decoder.JSONDecodeError:
                    print('msg `%s` from the server was invalid json' % msg)
            else:
                incoming = self.sock.recv(4096).decode('utf-8')
                buf += incoming

    def choose_action(self, game):
        """ 
            You're code goes here, you must return an action:
            'F' => Forward
            'R' => TurnRight
            'L' => TurnLeft
            'E' => Eat
            'S' => Stay
        """

        print('----' * 3)
        for y in range(4):
            for x in range(3):
                entity = 'M' if game[x][y]['mob'] else 'P' if game[x][y]['player'] else '_'
                print(' {}{} '.format(game[x][y]['base'], entity), end='')
            print('')
        print('----' * 3)

        return 'F'


if __name__ =='__main__':
    ai = Ai('localhost', 2010)

    ai.start()
