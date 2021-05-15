#import "Metal/Metal.h"

void say_hello_from_objc() {
    NSString* message = [NSString stringWithCString:"Hello from Objective-C!" encoding:NSASCIIStringEncoding];
    NSLog(@"%@", message);
}
