#include<Foundation/Foundation.h>
void hard_exception(void (fn)(void *context), void *context) {
    NSLog(@"hard_exception");
    @try {
        fn(context);
    }
    @catch (id ex) {
        NSLog(@"objc exception: %@",ex);
        abort();
    }
}