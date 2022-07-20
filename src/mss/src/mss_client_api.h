#ifndef __MSS_CLIENT_API_H__
#define __MSS_CLIENT_API_H__

#include <stdint.h>
#include <stddef.h>
/*-------------------*/
/* Client API to MSS */
/*-------------------*/

#ifdef __cplusplus
extern "C" {
#endif

  /* segment array element */
  typedef struct {
    uint64_t page_offset; // in pages
    uint64_t page_count;
  } segment_pair_t;

  /** 
   * Initial subsystem
   * 
   * @param lcores Worker cores (see DPDK)
   * 
   * @return 0 on success
   */
  int mss_init(const char * lcores);

  /** 
   * Allocate memory for snapshotting
   * 
   * @param size Size of memory in bytes
   * @param align Alignment in bytes
   * 
   * @return Pointer to allocated buffer or NULL on failure
   */
  void * mss_malloc(size_t size, size_t align);

  /** 
   * Free memory previously allocated with mss_malloc
   * 
   * @param ptr Pointer to buffer to free
   */
  void mss_free(void * ptr);

  /** 
   * Trigger snapshot
   * 
   * @param ptr Pointer to active buffer, allocated with mss_alloc
   * @param segment_array Segment array 
   * @param segment_array_count Segment array element count
   * 
   * @return 0 on succss, < 0 on error
   */
  int mss_snapshot(const void * ptr,
                   const segment_pair_t * segment_array,
                   const size_t segment_array_count);

  /** 
   * Fill gamma buffer (this is the last know state buffer)
   * 
   * @param active_ptr Active data buffer (to get ensemble)
   * @param data_ptr Data to copy from
   * @param segment_array Segment array
   * @param segment_array_count Segment array element count
   * 
   * @return 0 on succss, < 0 on error
   */
  int mss_fill_gamma(const void * active_ptr,
                     const void * data_ptr,
                     const segment_pair_t * segment_array,
                     const size_t segment_array_count);


  /** 
   * Allocate contiguous memory from DPDK subsystem
   * 
   * @param type Label for allocation
   * @param size Size in bytes
   * @param align Alignment in bytes
   * 
   * @return Pointer to allocated buffer
   */
  void * mss_rte_malloc(const char * type, size_t size, unsigned align);

  /** 
   * Free memory allocated from mss_rte_malloc
   * 
   * @param ptr Pointer to buffer to free
   */
  void mss_rte_free(void * ptr);


  /** 
   * Close down session and release resources
   * 
   */
  void mss_shutdown();

#ifdef __cplusplus
};
#endif

#endif // __MSS_CLIENT_API_H__
